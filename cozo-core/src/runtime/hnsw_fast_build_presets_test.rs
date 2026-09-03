/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Track 026: fast-build preset grid. Config measurement, not engine speedup.
//! 14k grid is `#[ignore]`; smoke stays in `nextest --lib`.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use ndarray::Array1;
use serde_json::{json, Value};

use crate::data::value::{DataValue, Vector};
use crate::runtime::hnsw_fixture::{
    hnsw_create_cfg, import_snippet_embedding_vecs, open_sqlite_temp, sqlite_db_bytes, sqlite_path,
    unit_normalized_vecs, HnswCreateCfg, FIXTURE_DIM, FIXTURE_N, FIXTURE_SEED,
};
use crate::{DbInstance, ScriptMutability};

const QUERY_SEED: u64 = FIXTURE_SEED.wrapping_add(1);
const QUERY_K: usize = 10;
const QUERY_EF: usize = 100;
const N_QUERIES_14K: usize = 50;
const OPTIONAL_M8_BUDGET: Duration = Duration::from_secs(90 * 60);

#[derive(Clone, Copy, Debug)]
struct GridCell {
    m: usize,
    ef_construction: usize,
    keep_pruned: bool,
    /// After the required cells, skip remaining m=8 lower-ef if the run is already long.
    required: bool,
}

const GRID: &[GridCell] = &[
    GridCell {
        m: 16,
        ef_construction: 100,
        keep_pruned: false,
        required: true,
    },
    GridCell {
        m: 16,
        ef_construction: 20,
        keep_pruned: false,
        required: true,
    },
    GridCell {
        m: 16,
        ef_construction: 40,
        keep_pruned: false,
        required: true,
    },
    GridCell {
        m: 16,
        ef_construction: 50,
        keep_pruned: false,
        required: true,
    },
    GridCell {
        m: 16,
        ef_construction: 40,
        keep_pruned: true,
        required: true,
    },
    GridCell {
        m: 8,
        ef_construction: 100,
        keep_pruned: false,
        required: true,
    },
    GridCell {
        m: 8,
        ef_construction: 50,
        keep_pruned: false,
        required: false,
    },
    GridCell {
        m: 8,
        ef_construction: 40,
        keep_pruned: false,
        required: false,
    },
    GridCell {
        m: 8,
        ef_construction: 20,
        keep_pruned: false,
        required: false,
    },
];

fn cell_label(c: &GridCell) -> String {
    if c.keep_pruned {
        format!("m{}-ef{}-keep_pruned", c.m, c.ef_construction)
    } else {
        format!("m{}-ef{}", c.m, c.ef_construction)
    }
}

fn maybe_write_json(filename: &str, value: &Value) {
    let Ok(dir) = std::env::var("COZO_HNSW_FAST_BUILD_OUT") else {
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

fn append_jsonl(filename: &str, value: &Value) {
    let Ok(dir) = std::env::var("COZO_HNSW_FAST_BUILD_OUT") else {
        return;
    };
    if dir.trim().is_empty() {
        return;
    }
    std::fs::create_dir_all(&dir).unwrap();
    let path = Path::new(&dir).join(filename);
    let mut line = serde_json::to_string(value).unwrap();
    line.push('\n');
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    f.write_all(line.as_bytes()).unwrap();
}

fn l2_sq(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

fn brute_top_k(corpus: &[Vec<f32>], query: &[f32], k: usize) -> Vec<i64> {
    let mut scored: Vec<(f32, i64)> = corpus
        .iter()
        .enumerate()
        .map(|(i, v)| (l2_sq(query, v), i as i64))
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored.into_iter().map(|(_, id)| id).collect()
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

fn run_grid(
    n: usize,
    dim: usize,
    n_queries: usize,
    k: usize,
    query_ef: usize,
    cells: &[GridCell],
    budget: Option<Duration>,
) -> Value {
    let corpus = unit_normalized_vecs(n, dim, FIXTURE_SEED);
    let queries = unit_normalized_vecs(n_queries, dim, QUERY_SEED);
    let brute: Vec<Vec<i64>> = queries.iter().map(|q| brute_top_k(&corpus, q, k)).collect();

    let mut baseline_hits: Option<Vec<Vec<i64>>> = None;
    let mut conversion_recall: Option<f64> = None;
    let mut rows: Vec<Value> = Vec::new();
    let run_start = Instant::now();
    let mut skipped: Vec<String> = Vec::new();

    for cell in cells {
        if !cell.required {
            if let Some(limit) = budget {
                if run_start.elapsed() > limit {
                    skipped.push(cell_label(cell));
                    eprintln!(
                        "skip {} (elapsed {:?} > budget {:?})",
                        cell_label(cell),
                        run_start.elapsed(),
                        limit
                    );
                    continue;
                }
            }
        }

        // Fresh sqlite per cell so DB bytes are comparable (no leftover pages after drop).
        let (tmp, db) = open_sqlite_temp();
        let db_path = sqlite_path(&tmp);
        import_snippet_embedding_vecs(&db, &corpus, dim);

        let cfg = HnswCreateCfg {
            dim,
            m: cell.m,
            ef_construction: cell.ef_construction,
            keep_pruned_connections: cell.keep_pruned,
            extend_candidates: false,
        };
        let t0 = Instant::now();
        hnsw_create_cfg(&db, cfg);
        let create_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let db_bytes = sqlite_db_bytes(&db_path);

        let hits = query_all(&db, &queries, k, query_ef);
        drop(db);
        drop(tmp);
        let recall_vs_brute = mean_recall(&hits, &brute, k);
        let recall_vs_baseline = if let Some(base) = baseline_hits.as_ref() {
            mean_recall(&hits, base, k)
        } else {
            1.0
        };

        if baseline_hits.is_none() {
            conversion_recall = Some(recall_vs_brute);
            baseline_hits = Some(hits);
        }

        let row = json!({
            "label": cell_label(cell),
            "m": cell.m,
            "ef_construction": cell.ef_construction,
            "keep_pruned_connections": cell.keep_pruned,
            "extend_candidates": false,
            "create_ms": create_ms,
            "db_bytes": db_bytes,
            "recall_at_k_vs_baseline": recall_vs_baseline,
            "recall_at_k_vs_brute": recall_vs_brute,
            "k": k,
            "query_ef": query_ef,
            "n_queries": n_queries,
        });
        eprintln!("cell {}", row);
        append_jsonl("cells.jsonl", &row);
        rows.push(row);
    }

    json!({
        "n": n,
        "dim": dim,
        "seed": FIXTURE_SEED,
        "query_seed": QUERY_SEED,
        "k": k,
        "query_ef": query_ef,
        "n_queries": n_queries,
        "baseline": "m:16,ef_construction:100",
        "conversion_row": {
            "index": "m:16,ef_construction:100",
            "query_ef": query_ef,
            "recall_at_k_vs_brute": conversion_recall,
        },
        "skipped": skipped,
        "cells": rows,
    })
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_fast_build_presets_smoke() {
    let smoke_cells = &[
        GridCell {
            m: 16,
            ef_construction: 16,
            keep_pruned: false,
            required: true,
        },
        GridCell {
            m: 16,
            ef_construction: 8,
            keep_pruned: false,
            required: true,
        },
        GridCell {
            m: 8,
            ef_construction: 16,
            keep_pruned: true,
            required: true,
        },
    ];
    let out = run_grid(48, 8, 8, 5, 16, smoke_cells, None);
    let cells = out["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 3);
    for cell in cells {
        let create_ms = cell["create_ms"].as_f64().unwrap();
        let db_bytes = cell["db_bytes"].as_u64().unwrap();
        let vs_base = cell["recall_at_k_vs_baseline"].as_f64().unwrap();
        let vs_brute = cell["recall_at_k_vs_brute"].as_f64().unwrap();
        assert!(create_ms > 0.0, "create_ms should be positive: {cell}");
        assert!(db_bytes > 0, "db_bytes should be positive: {cell}");
        assert!((0.0..=1.0).contains(&vs_base), "recall vs baseline: {cell}");
        assert!((0.0..=1.0).contains(&vs_brute), "recall vs brute: {cell}");
    }
    let conv = out["conversion_row"]["recall_at_k_vs_brute"]
        .as_f64()
        .unwrap();
    assert!(
        conv >= 0.99,
        "tiny corpus should recover brute top-k, got {conv}: {out}"
    );
}

#[cfg(feature = "storage-sqlite")]
#[ignore]
#[test]
fn hnsw_fast_build_presets_14k() {
    let out = run_grid(
        FIXTURE_N,
        FIXTURE_DIM,
        N_QUERIES_14K,
        QUERY_K,
        QUERY_EF,
        GRID,
        Some(OPTIONAL_M8_BUDGET),
    );
    eprintln!("grid {}", out);
    maybe_write_json("grid.json", &out);
    let cells = out["cells"].as_array().expect("cells array");
    assert!(
        cells.iter().any(|c| {
            c["m"] == 16 && c["ef_construction"] == 20 && c["keep_pruned_connections"] == false
        }),
        "kill probe m=16 ef=20 must run: {out}"
    );
    assert!(
        cells
            .iter()
            .any(|c| c["m"] == 16 && c["ef_construction"] == 100),
        "baseline m=16 ef=100 must run: {out}"
    );
    assert!(
        cells.iter().any(|c| {
            c["m"] == 16 && c["ef_construction"] == 40 && c["keep_pruned_connections"] == true
        }),
        "keep_pruned (16,40) cell must run: {out}"
    );
    assert_eq!(
        out["n"].as_u64().expect("n"),
        FIXTURE_N as u64,
        "14k grid must keep N={FIXTURE_N}: {out}"
    );
    assert!(
        out["conversion_row"]["recall_at_k_vs_brute"]
            .as_f64()
            .is_some(),
        "conversion row vs brute must be a number: {out}"
    );
}
