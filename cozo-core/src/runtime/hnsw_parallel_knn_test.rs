/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Track 029: parallel `HnswSearchRA` parent-tuple k-NN on concurrent-read-safe backends.

use std::collections::BTreeMap;

use ndarray::Array1;

use crate::data::value::{DataValue, Vector};
use crate::runtime::hnsw_fixture::open_sqlite_temp;
use crate::{DbInstance, NamedRows, ScriptMutability};

const DIM: usize = 8;
const CORPUS_N: usize = 32;
const QUERY_N: usize = 12;
const K: usize = 3;
const EF: usize = 32;

fn vec_dv(vals: &[f32]) -> DataValue {
    DataValue::Vec(Box::new(Vector::F32(Array1::from(vals.to_vec()))))
}

fn corpus_vec(i: usize) -> Vec<f32> {
    (0..DIM)
        .map(|d| ((i * 7 + d * 3) % 17) as f32 / 17.0)
        .collect()
}

fn query_vec(i: usize) -> Vec<f32> {
    let mut v = corpus_vec(i);
    v[0] += 0.01;
    v
}

fn load_parent_knn_fixture(db: &DbInstance) {
    db.run_default(&format!(
        ":create docs {{id: Int => embedding: <F32; {DIM}>}}"
    ))
    .unwrap();
    db.run_default(&format!(":create queries {{qid: Int => qv: <F32; {DIM}>}}"))
        .unwrap();

    let doc_rows: Vec<Vec<DataValue>> = (0..CORPUS_N)
        .map(|i| vec![DataValue::from(i as i64), vec_dv(&corpus_vec(i))])
        .collect();
    let query_rows: Vec<Vec<DataValue>> = (0..QUERY_N)
        .map(|i| vec![DataValue::from(i as i64), vec_dv(&query_vec(i))])
        .collect();

    db.import_relations(BTreeMap::from([
        (
            "docs".to_string(),
            NamedRows {
                headers: vec!["id".to_string(), "embedding".to_string()],
                rows: doc_rows.into_iter().map(Into::into).collect(),
                next: None,
            },
        ),
        (
            "queries".to_string(),
            NamedRows {
                headers: vec!["qid".to_string(), "qv".to_string()],
                rows: query_rows.into_iter().map(Into::into).collect(),
                next: None,
            },
        ),
    ]))
    .unwrap();

    db.run_default(
        r#"
        ::hnsw create docs:idx {
            fields: [embedding],
            dim: 8,
            m: 8,
            ef_construction: 40,
            distance: L2,
        }
    "#,
    )
    .unwrap();
}

fn parent_knn_script(filter: &str, limit: Option<usize>) -> String {
    let filter_clause = if filter.is_empty() {
        String::new()
    } else {
        format!(", filter: {filter}")
    };
    let limit_clause = match limit {
        Some(n) => format!("\n:limit {n}"),
        None => String::new(),
    };
    format!(
        "?[qid, id, dist] := *queries{{qid, qv}}, ~docs:idx{{id | query: qv, k: {K}, ef: {EF}, bind_distance: dist{filter_clause}}}{limit_clause}"
    )
}

fn run_parent_knn(db: &DbInstance, filter: &str, limit: Option<usize>) -> Vec<(i64, i64, f64)> {
    let res = db
        .run_script(
            &parent_knn_script(filter, limit),
            BTreeMap::new(),
            ScriptMutability::Immutable,
        )
        .unwrap();
    let mut rows: Vec<(i64, i64, f64)> = res
        .rows
        .iter()
        .map(|row| {
            (
                row[0].get_int().expect("qid"),
                row[1].get_int().expect("id"),
                row[2].get_float().expect("dist"),
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    rows
}

fn neighbor_ids(rows: &[(i64, i64, f64)]) -> Vec<(i64, i64)> {
    rows.iter().map(|(q, id, _)| (*q, *id)).collect()
}

#[test]
fn mem_parent_knn_parallel_matches_sequential_on_same_db() {
    let db = DbInstance::new("mem", "", "").unwrap();
    load_parent_knn_fixture(&db);
    let parallel = run_parent_knn(&db, "", None);
    assert_eq!(parallel.len(), QUERY_N * K);
    let sequential = db.run_default(&parent_knn_script("", None)).unwrap();
    let mut seq_rows: Vec<(i64, i64, f64)> = sequential
        .rows
        .iter()
        .map(|row| {
            (
                row[0].get_int().expect("qid"),
                row[1].get_int().expect("id"),
                row[2].get_float().expect("dist"),
            )
        })
        .collect();
    seq_rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    assert_eq!(neighbor_ids(&parallel), neighbor_ids(&seq_rows));
    for ((_, _, pd), (_, _, sd)) in parallel.iter().zip(seq_rows.iter()) {
        assert!((pd - sd).abs() < 1e-6, "dist {pd} vs {sd}");
    }
}

#[test]
fn mem_parent_knn_filter_even_ids() {
    let db = DbInstance::new("mem", "", "").unwrap();
    load_parent_knn_fixture(&db);
    let rows = run_parent_knn(&db, "id % 2 == 0", None);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|(_, id, _)| id % 2 == 0));
    assert_eq!(rows.len(), QUERY_N * K);
}

#[test]
fn mem_parent_knn_limit_stops_at_five() {
    let db = DbInstance::new("mem", "", "").unwrap();
    load_parent_knn_fixture(&db);
    let rows = run_parent_knn(&db, "", Some(5));
    assert_eq!(rows.len(), 5);
}

#[test]
fn mem_mutable_parent_knn_still_works() {
    let db = DbInstance::new("mem", "", "").unwrap();
    load_parent_knn_fixture(&db);
    let res = db.run_default(&parent_knn_script("", None)).unwrap();
    assert_eq!(res.rows.len(), QUERY_N * K);
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn sqlite_parent_knn_works_without_concurrent_reads() {
    let (_tmp, db) = open_sqlite_temp();
    load_parent_knn_fixture(&db);
    let rows = run_parent_knn(&db, "", None);
    assert_eq!(rows.len(), QUERY_N * K);
}

#[cfg(feature = "storage-fjall")]
#[test]
fn fjall_parent_knn_parallel_matches_sequential() {
    let tmp = tempfile::tempdir().unwrap();
    let db = DbInstance::new("fjall", tmp.path(), "").unwrap();
    load_parent_knn_fixture(&db);
    let parallel = run_parent_knn(&db, "", None);
    let sequential = db.run_default(&parent_knn_script("", None)).unwrap();
    assert_eq!(parallel.len(), QUERY_N * K);
    assert_eq!(sequential.rows.len(), QUERY_N * K);
}

#[cfg(feature = "storage-rocksdb")]
#[test]
fn rocksdb_parent_knn_parallel_matches_sequential() {
    let tmp = tempfile::tempdir().unwrap();
    let db = DbInstance::new("rocksdb", tmp.path(), "").unwrap();
    load_parent_knn_fixture(&db);
    let parallel = run_parent_knn(&db, "", None);
    let sequential = db.run_default(&parent_knn_script("", None)).unwrap();
    assert_eq!(parallel.len(), QUERY_N * K);
    assert_eq!(sequential.rows.len(), QUERY_N * K);
}
