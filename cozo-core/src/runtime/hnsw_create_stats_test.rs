/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use crate::runtime::hnsw_create_stats::{self, HnswCreateStatsSnapshot};
use crate::runtime::hnsw_fixture::{
    hnsw_create, hnsw_drop, import_snippet_embeddings, open_sqlite_temp, FIXTURE_DIM, FIXTURE_N,
    FIXTURE_SEED,
};

fn enable_create_stats() {
    // SAFETY: this module's tests set a process env flag only around HNSW create
    // measurement. Callers must reset() after this, and Drop guards unset it.
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

fn maybe_write_snapshot(snap: &HnswCreateStatsSnapshot, filename: &str) {
    let Ok(dir) = std::env::var("COZO_HNSW_CREATE_STATS_OUT") else {
        return;
    };
    if dir.trim().is_empty() {
        return;
    }
    std::fs::create_dir_all(&dir).unwrap();
    let path = Path::new(&dir).join(filename);
    let pretty = serde_json::to_string_pretty(&snap.to_json_value()).unwrap();
    std::fs::write(path, pretty).unwrap();
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_create_stats_smoke() {
    let _guard = StatsEnvGuard;
    enable_create_stats();
    let (_tmp, db) = open_sqlite_temp();
    import_snippet_embeddings(&db, 32, 8, FIXTURE_SEED);
    hnsw_create(&db, 8, 16);
    let snap = hnsw_create_stats::take();
    assert!(
        snap.put_count > 0,
        "expected store puts during create, got {snap:?}"
    );
    assert!(
        snap.dist_ns > 0 || snap.dist_batches > 0,
        "expected distance work during create, got {snap:?}"
    );
    assert!(
        snap.cache_instances > 0,
        "expected VectorCache instances on the hnsw_put path, got {snap:?}"
    );
}

#[cfg(feature = "storage-sqlite")]
#[ignore]
#[test]
fn hnsw_create_baseline_14k() {
    let _guard = StatsEnvGuard;
    enable_create_stats();
    let (_tmp, db) = open_sqlite_temp();
    import_snippet_embeddings(&db, FIXTURE_N, FIXTURE_DIM, FIXTURE_SEED);

    hnsw_create(&db, FIXTURE_DIM, 50);
    let ef50 = hnsw_create_stats::take();
    eprintln!("ef50 {}", ef50.to_json_value());
    maybe_write_snapshot(&ef50, "ef50.json");

    hnsw_drop(&db);
    enable_create_stats();
    hnsw_create(&db, FIXTURE_DIM, 100);
    let ef100 = hnsw_create_stats::take();
    eprintln!("ef100 {}", ef100.to_json_value());
    maybe_write_snapshot(&ef100, "ef100.json");

    assert!(ef50.put_count > 0 && ef100.put_count > 0);
    assert!(ef50.cache_instances > 0 && ef100.cache_instances > 0);
    assert!(
        ef50.dist_ns > 0 || ef50.dist_batches > 0 || ef100.dist_ns > 0 || ef100.dist_batches > 0
    );
}
