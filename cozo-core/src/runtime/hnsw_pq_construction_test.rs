/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Track 027: PQ train_pq vs L2 create, plus L2 guard / centroid bound / re-rank.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use ndarray::Array1;
use serde_json::{json, Value};

use crate::data::value::{DataValue, Vector};
use crate::runtime::hnsw::{encode_vector_pq, PqCodebook};
use crate::runtime::hnsw_create_stats::{self, HnswCreateStatsSnapshot};
use crate::runtime::hnsw_fixture::{
    hnsw_create, import_snippet_embedding_vecs, open_sqlite_temp, unit_normalized_vecs,
    FIXTURE_DIM, FIXTURE_N, FIXTURE_SEED,
};
use crate::{DbInstance, ScriptMutability};

const QUERY_SEED: u64 = FIXTURE_SEED.wrapping_add(1);

fn maybe_write_json(filename: &str, value: &Value) {
    let Ok(dir) = std::env::var("COZO_HNSW_PQ_OUT") else {
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

fn enable_create_stats() {
    unsafe {
        std::env::set_var("COZO_HNSW_CREATE_STATS", "1");
    }
    hnsw_create_stats::reset();
}

fn disable_create_stats() {
    unsafe {
        std::env::remove_var("COZO_HNSW_CREATE_STATS");
    }
    hnsw_create_stats::reset();
}

struct StatsEnvGuard;

impl Drop for StatsEnvGuard {
    fn drop(&mut self) {
        disable_create_stats();
    }
}

fn train_pq(db: &DbInstance, subspaces: usize, centroids: usize, samples: usize) {
    db.run_default(&format!(
        "::hnsw train_pq snippet_embedding:snippet_idx {{ subspaces: {subspaces}, centroids: {centroids}, samples: {samples} }}"
    ))
    .unwrap();
}

fn estimate_encode_ms(corpus: &[Vec<f32>], subspaces: usize, centroids: usize) -> f64 {
    let dim = corpus[0].len();
    assert_eq!(dim % subspaces, 0);
    let sub_dim = dim / subspaces;
    let codebook = PqCodebook {
        num_subspaces: subspaces,
        num_centroids: centroids,
        sub_dim,
        centroids: vec![0.0f32; subspaces * centroids * sub_dim],
    };
    let t = Instant::now();
    for v in corpus {
        let dv = DataValue::Vec(Box::new(Vector::F32(Array1::from(v.clone()))));
        let DataValue::Vec(boxed) = dv else {
            panic!("expected vec");
        };
        encode_vector_pq(boxed.as_ref(), &codebook).unwrap();
    }
    t.elapsed().as_secs_f64() * 1000.0
}

fn estimate_lut_ms(queries: &[Vec<f32>], subspaces: usize, centroids: usize) -> f64 {
    let dim = queries[0].len();
    let sub_dim = dim / subspaces;
    let centroids_flat = vec![0.0f32; subspaces * centroids * sub_dim];
    let t = Instant::now();
    for q in queries {
        let mut table = vec![vec![0.0f64; centroids]; subspaces];
        for (m, table_m) in table.iter_mut().enumerate() {
            let start = m * sub_dim;
            let q_sub = &q[start..start + sub_dim];
            for (c, cell) in table_m.iter_mut().enumerate() {
                let c_start = (m * centroids + c) * sub_dim;
                let centroid = &centroids_flat[c_start..c_start + sub_dim];
                let dist: f32 = q_sub
                    .iter()
                    .zip(centroid.iter())
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum();
                *cell = dist as f64;
            }
        }
    }
    t.elapsed().as_secs_f64() * 1000.0
}

fn run_gate_a_b(
    n: usize,
    dim: usize,
    subspaces: usize,
    centroids: usize,
    samples: usize,
    n_queries: usize,
) -> Value {
    let corpus = unit_normalized_vecs(n, dim, FIXTURE_SEED);
    let queries = unit_normalized_vecs(n_queries, dim, QUERY_SEED);

    hnsw_create_stats::with_exclusive(|| {
        let _guard = StatsEnvGuard;
        enable_create_stats();
        let (_tmp, db) = open_sqlite_temp();
        import_snippet_embedding_vecs(&db, &corpus, dim);
        let t_create = Instant::now();
        hnsw_create(&db, dim, 100);
        let create_ms = t_create.elapsed().as_secs_f64() * 1000.0;
        let snap: HnswCreateStatsSnapshot = hnsw_create_stats::take();
        disable_create_stats();

        let t_train = Instant::now();
        train_pq(&db, subspaces, centroids, samples);
        let train_ms = t_train.elapsed().as_secs_f64() * 1000.0;

        let encode_ms = estimate_encode_ms(&corpus, subspaces, centroids);
        let lut_ms = estimate_lut_ms(&queries, subspaces, centroids);
        let dist_ms = snap.dist_ns as f64 / 1_000_000.0;
        let create_total_ms = snap.create_total_ns as f64 / 1_000_000.0;
        let dist_pct = if create_total_ms > 0.0 {
            100.0 * dist_ms / create_total_ms
        } else {
            0.0
        };
        // Construction-PQ can only replace v_dist (not k_dist). Kill if encode+LUT
        // is not cheaper than dist, or dist is a small share of create.
        let keep_construction_pq = encode_ms + lut_ms < dist_ms && dist_pct >= 25.0;

        json!({
            "n": n,
            "dim": dim,
            "subspaces": subspaces,
            "centroids": centroids,
            "samples": samples,
            "create_wall_ms": create_ms,
            "create_instrumented_ms": create_total_ms,
            "dist_ms": dist_ms,
            "dist_pct": dist_pct,
            "train_pq_ms": train_ms,
            "encode_estimate_ms": encode_ms,
            "lut_ms": lut_ms,
            "keep_construction_pq": keep_construction_pq,
        })
    })
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_train_pq_rejects_cosine() {
    let (_tmp, db) = open_sqlite_temp();
    let corpus = unit_normalized_vecs(16, 8, FIXTURE_SEED);
    import_snippet_embedding_vecs(&db, &corpus, 8);
    db.run_default(
        "::hnsw create snippet_embedding:snippet_idx { dim: 8, dtype: F32, fields: [embedding], distance: Cosine, m: 8, ef_construction: 16 }",
    )
    .unwrap();
    let err = db
        .run_default(
            "::hnsw train_pq snippet_embedding:snippet_idx { subspaces: 2, centroids: 4, samples: 16 }",
        )
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("L2"),
        "cosine train_pq must fail L2 guard, got {err}"
    );
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_train_pq_rejects_centroid_overflow() {
    let (_tmp, db) = open_sqlite_temp();
    let corpus = unit_normalized_vecs(16, 8, FIXTURE_SEED);
    import_snippet_embedding_vecs(&db, &corpus, 8);
    hnsw_create(&db, 8, 16);
    let err = db
        .run_default(
            "::hnsw train_pq snippet_embedding:snippet_idx { subspaces: 2, centroids: 257, samples: 16 }",
        )
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("256") || err.contains("centroids"),
        "centroids 257 must fail, got {err}"
    );
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_pq_search_reranks_with_exact_l2() {
    let (_tmp, db) = open_sqlite_temp();
    let corpus = unit_normalized_vecs(48, 8, FIXTURE_SEED);
    import_snippet_embedding_vecs(&db, &corpus, 8);
    hnsw_create(&db, 8, 50);
    train_pq(&db, 2, 4, 48);
    let q = &corpus[0];
    let qv = DataValue::Vec(Box::new(Vector::F32(Array1::from(q.to_vec()))));
    let res = db
        .run_script(
            "?[id, dist] := ~snippet_embedding:snippet_idx{id | query: $q, k: 5, ef: 20, bind_distance: dist}\n:order dist",
            BTreeMap::from([("q".to_string(), qv)]),
            ScriptMutability::Immutable,
        )
        .unwrap();
    assert_eq!(res.rows.len(), 5);
    for row in &res.rows {
        let id = row[0].get_int().expect("id") as usize;
        let reported = row[1].get_float().expect("dist is float");
        let exact: f32 = q
            .iter()
            .zip(corpus[id].iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum();
        let delta = (reported - exact as f64).abs();
        assert!(
            delta < 1e-4,
            "bind_distance must be exact L2 after re-rank, id={id} reported={reported} exact={exact} delta={delta}"
        );
    }
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_pq_gates_smoke() {
    let report = run_gate_a_b(48, 8, 2, 4, 48, 8);
    eprintln!("pq_smoke {report}");
    maybe_write_json("pq_smoke.json", &report);
    assert!(report["create_wall_ms"].as_f64().unwrap() > 0.0);
    assert!(report["train_pq_ms"].as_f64().unwrap() > 0.0);
}

#[cfg(feature = "storage-sqlite")]
#[ignore]
#[test]
fn hnsw_pq_gates_14k() {
    // Default train_pq knobs: subspaces 8, centroids 256, samples 10000.
    let report = run_gate_a_b(FIXTURE_N, FIXTURE_DIM, 8, 256, 10_000, 50);
    eprintln!("pq_14k {report}");
    maybe_write_json("pq_14k.json", &report);
    assert!(report["create_wall_ms"].as_f64().unwrap() > 0.0);
}
