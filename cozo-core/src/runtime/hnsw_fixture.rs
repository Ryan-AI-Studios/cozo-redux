/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Shared Ledgerful-shaped HNSW fixture for tracks 021 and 026.
//! Seed 21768, dim 768, N=14000, unit-normalized F32, SQLite tempfile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ndarray::Array1;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::data::value::{DataValue, Vector};
use crate::{DbInstance, NamedRows};

pub(super) const FIXTURE_SEED: u64 = 21_768;
pub(super) const FIXTURE_DIM: usize = 768;
pub(super) const FIXTURE_N: usize = 14_000;
pub(super) const SQLITE_FILENAME: &str = "hnsw-create.sqlite";

#[derive(Clone, Copy, Debug)]
pub(super) struct HnswCreateCfg {
    pub dim: usize,
    pub m: usize,
    pub ef_construction: usize,
    pub keep_pruned_connections: bool,
    pub extend_candidates: bool,
}

impl HnswCreateCfg {
    pub(super) fn m16(dim: usize, ef_construction: usize) -> Self {
        Self {
            dim,
            m: 16,
            ef_construction,
            keep_pruned_connections: false,
            extend_candidates: false,
        }
    }
}

pub(super) fn unit_normalized_vecs(n: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n)
        .map(|_| {
            let mut v: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() * 2.0 - 1.0).collect();
            let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut v {
                    *x /= norm;
                }
            }
            v
        })
        .collect()
}

pub(super) fn rows_from_vecs(vecs: &[Vec<f32>]) -> Vec<Vec<DataValue>> {
    vecs.iter()
        .enumerate()
        .map(|(i, v)| {
            vec![
                DataValue::from(i as i64),
                DataValue::Vec(Box::new(Vector::F32(Array1::from(v.clone())))),
            ]
        })
        .collect()
}

pub(super) fn unit_normalized_rows(n: usize, dim: usize, seed: u64) -> Vec<Vec<DataValue>> {
    rows_from_vecs(&unit_normalized_vecs(n, dim, seed))
}

pub(super) fn open_sqlite_temp() -> (tempfile::TempDir, DbInstance) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(SQLITE_FILENAME);
    let db = DbInstance::new("sqlite", &path, "").unwrap();
    (tmp, db)
}

pub(super) fn sqlite_path(tmp: &tempfile::TempDir) -> PathBuf {
    tmp.path().join(SQLITE_FILENAME)
}

/// Main sqlite file plus `-wal` / `-shm` if present.
pub(super) fn sqlite_db_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    for suffix in ["", "-wal", "-shm"] {
        let p = if suffix.is_empty() {
            path.to_path_buf()
        } else {
            let mut s = path.as_os_str().to_os_string();
            s.push(suffix);
            PathBuf::from(s)
        };
        if let Ok(meta) = std::fs::metadata(&p) {
            total += meta.len();
        }
    }
    total
}

pub(super) fn import_snippet_embeddings(db: &DbInstance, n: usize, dim: usize, seed: u64) {
    create_and_import_snippet_embeddings(db, dim, unit_normalized_rows(n, dim, seed));
}

pub(super) fn import_snippet_embedding_vecs(db: &DbInstance, vecs: &[Vec<f32>], dim: usize) {
    create_and_import_snippet_embeddings(db, dim, rows_from_vecs(vecs));
}

fn create_and_import_snippet_embeddings(db: &DbInstance, dim: usize, rows: Vec<Vec<DataValue>>) {
    db.run_default(&format!(
        ":create snippet_embedding {{id: Int => embedding: <F32; {dim}>}}"
    ))
    .unwrap();
    db.import_relations(BTreeMap::from([(
        "snippet_embedding".to_string(),
        NamedRows {
            headers: vec!["id".to_string(), "embedding".to_string()],
            rows: rows.into_iter().map(Into::into).collect(),
            next: None,
        },
    )]))
    .unwrap();
}

pub(super) fn hnsw_create(db: &DbInstance, dim: usize, ef_construction: usize) {
    hnsw_create_cfg(db, HnswCreateCfg::m16(dim, ef_construction));
}

pub(super) fn hnsw_create_cfg(db: &DbInstance, cfg: HnswCreateCfg) {
    let keep = if cfg.keep_pruned_connections {
        ", keep_pruned_connections: true"
    } else {
        ""
    };
    let extend = if cfg.extend_candidates {
        ", extend_candidates: true"
    } else {
        ""
    };
    db.run_default(&format!(
        "::hnsw create snippet_embedding:snippet_idx {{ dim: {}, dtype: F32, fields: [embedding], distance: L2, m: {}, ef_construction: {}{keep}{extend} }}",
        cfg.dim, cfg.m, cfg.ef_construction
    ))
    .unwrap();
}

pub(super) fn hnsw_drop(db: &DbInstance) {
    db.run_default("::hnsw drop snippet_embedding:snippet_idx")
        .unwrap();
}
