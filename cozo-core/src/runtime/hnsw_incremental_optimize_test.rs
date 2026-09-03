/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Track 025: live-index incremental appends vs drop+create rebuild.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Instant;

use ndarray::Array1;
use serde_json::{json, Value};

use crate::data::value::{DataValue, Vector};
use crate::runtime::hnsw_fixture::{
    hnsw_create, hnsw_drop, import_snippet_embedding_vecs, open_sqlite_temp, unit_normalized_vecs,
    FIXTURE_DIM, FIXTURE_N, FIXTURE_SEED,
};
use crate::{DbInstance, NamedRows, ScriptMutability};

const QUERY_SEED: u64 = FIXTURE_SEED.wrapping_add(1);
const QUERY_K: usize = 10;
const QUERY_EF: usize = 100;
const RECALL_BAND: f64 = 0.90;

fn maybe_write_json(filename: &str, value: &Value) {
    let Ok(dir) = std::env::var("COZO_HNSW_INCREMENTAL_OUT") else {
        return;
    };
    if dir.trim().is_empty() {
        return;
    }
    std::fs::create_dir_all(&dir).unwrap();
    let path = Path::new(&dir).join(filename);
    let pretty = serde_json::to_string_pretty(value).unwrap();
    std::fs::write(path, pretty).unwrap();
}

fn hnsw_top_k(db: &DbInstance, query: &[f32], k: usize, ef: usize) -> Vec<i64> {
    let q = DataValue::Vec(Box::new(Vector::F32(Array1::from(query.to_vec()))));
    let script = format!(
        "?[id, dist] := ~snippet_embedding:snippet_idx{{id | query: $q, k: {k}, ef: {ef}, bind_distance: dist}}\n:order dist"
    );
    let res = db
        .run_script(
            &script,
            BTreeMap::from([("q".to_string(), q)]),
            ScriptMutability::Immutable,
        )
        .unwrap();
    res.rows
        .iter()
        .map(|row| row[0].get_int().expect("hnsw search id is int"))
        .collect()
}

fn recall_at_k(got: &[i64], truth: &[i64], k: usize) -> f64 {
    let truth_set: HashSet<i64> = truth.iter().take(k).copied().collect();
    if k == 0 {
        return 0.0;
    }
    let hits = got
        .iter()
        .take(k)
        .filter(|id| truth_set.contains(id))
        .count();
    hits as f64 / k as f64
}

fn mean_recall(got_lists: &[Vec<i64>], truth_lists: &[Vec<i64>], k: usize) -> f64 {
    assert_eq!(got_lists.len(), truth_lists.len());
    if got_lists.is_empty() {
        return 0.0;
    }
    let sum: f64 = got_lists
        .iter()
        .zip(truth_lists.iter())
        .map(|(g, t)| recall_at_k(g, t, k))
        .sum();
    sum / got_lists.len() as f64
}

fn query_all(db: &DbInstance, queries: &[Vec<f32>], k: usize, ef: usize) -> Vec<Vec<i64>> {
    queries.iter().map(|q| hnsw_top_k(db, q, k, ef)).collect()
}

fn indexed_node_count(db: &DbInstance) -> usize {
    let res = db
        .run_default("?[fr_id] := *snippet_embedding:snippet_idx{layer: 0, fr_id}")
        .unwrap();
    let ids: HashSet<i64> = res.rows.iter().filter_map(|row| row[0].get_int()).collect();
    ids.len()
}

/// One mutable `:put` with `$data` for the whole range — Ledgerful's live-append cadence.
fn put_range(db: &DbInstance, corpus: &[Vec<f32>], start: usize, end: usize) -> usize {
    let before = indexed_node_count(db);
    let rows: Vec<crate::data::tuple::Tuple> = corpus
        .iter()
        .enumerate()
        .take(end)
        .skip(start)
        .map(|(i, v)| {
            vec![
                DataValue::from(i as i64),
                DataValue::Vec(Box::new(Vector::F32(Array1::from(v.clone())))),
            ]
            .into()
        })
        .collect();
    let (script, params) = NamedRows {
        headers: vec!["id".to_string(), "embedding".to_string()],
        rows,
        next: None,
    }
    .into_payload("snippet_embedding", "put");
    db.run_script(&script, params, ScriptMutability::Mutable)
        .unwrap();
    let after = indexed_node_count(db);
    let inserted = after.saturating_sub(before);
    assert_eq!(
        inserted,
        end - start,
        "incremental hnsw_put must insert new graph nodes (canary no-op?), before={before} after={after} range={start}..{end}"
    );
    inserted
}

fn run_a_vs_b2(n: usize, dim: usize, n0: usize, batch: usize, n_queries: usize) -> Value {
    assert!(n0 < n);
    assert_eq!((n - n0) % batch, 0);
    let corpus = unit_normalized_vecs(n, dim, FIXTURE_SEED);
    let queries = unit_normalized_vecs(n_queries, dim, QUERY_SEED);

    let (_tmp_a, db_a) = open_sqlite_temp();
    import_snippet_embedding_vecs(&db_a, &corpus, dim);
    let t_a = Instant::now();
    hnsw_create(&db_a, dim, 100);
    let a_ms = t_a.elapsed().as_secs_f64() * 1000.0;
    let a_nodes = indexed_node_count(&db_a);
    assert_eq!(a_nodes, n, "build A must index all rows");
    let a_hits = query_all(&db_a, &queries, QUERY_K, QUERY_EF);
    drop(db_a);

    let (_tmp_b, db_b) = open_sqlite_temp();
    import_snippet_embedding_vecs(&db_b, &corpus[..n0], dim);
    let t_b = Instant::now();
    hnsw_create(&db_b, dim, 100);
    let b_create_ms = t_b.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(indexed_node_count(&db_b), n0);

    let t_append = Instant::now();
    let mut cursor = n0;
    let mut batches = 0u64;
    while cursor < n {
        let end = cursor + batch;
        put_range(&db_b, &corpus, cursor, end);
        cursor = end;
        batches += 1;
    }
    let append_ms = t_append.elapsed().as_secs_f64() * 1000.0;
    let b_total_ms = b_create_ms + append_ms;
    assert_eq!(indexed_node_count(&db_b), n);
    let b_hits = query_all(&db_b, &queries, QUERY_K, QUERY_EF);
    drop(db_b);

    let recall_vs_a = mean_recall(&b_hits, &a_hits, QUERY_K);
    json!({
        "n": n,
        "dim": dim,
        "n0": n0,
        "batch": batch,
        "batches": batches,
        "n_queries": n_queries,
        "k": QUERY_K,
        "query_ef": QUERY_EF,
        "build_a_create_ms": a_ms,
        "build_b2_create_n0_ms": b_create_ms,
        "build_b2_append_ms": append_ms,
        "build_b2_total_ms": b_total_ms,
        "recall_at_10_vs_a": recall_vs_a,
        "keep_band": RECALL_BAND,
        "keep_quality": recall_vs_a >= RECALL_BAND,
        "put_mode": "one_script_per_batch",
        "rows_per_put_script": batch,
    })
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_incremental_b2_smoke() {
    let report = run_a_vs_b2(48, 8, 24, 8, 8);
    eprintln!("incremental_smoke {report}");
    maybe_write_json("incremental_smoke.json", &report);
    assert!(
        report["keep_quality"].as_bool().unwrap(),
        "smoke B2 should match A on dim-8, got {report}"
    );
}

#[cfg(feature = "storage-sqlite")]
#[ignore]
#[test]
fn hnsw_incremental_b2_14k() {
    // Ledgerful cadence: rebuild at 500. B2 = create 10k then 8 live appends of 500.
    let report = run_a_vs_b2(FIXTURE_N, FIXTURE_DIM, 10_000, 500, 50);
    eprintln!("incremental_14k {report}");
    maybe_write_json("incremental_14k.json", &report);
    assert!(report["build_a_create_ms"].as_f64().unwrap() > 0.0);
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_drop_create_still_works() {
    let (_tmp, db) = open_sqlite_temp();
    let corpus = unit_normalized_vecs(16, 8, FIXTURE_SEED);
    import_snippet_embedding_vecs(&db, &corpus, 8);
    hnsw_create(&db, 8, 16);
    hnsw_drop(&db);
    hnsw_create(&db, 8, 16);
    assert_eq!(indexed_node_count(&db), 16);
}
