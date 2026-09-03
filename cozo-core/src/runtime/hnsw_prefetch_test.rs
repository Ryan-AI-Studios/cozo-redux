/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Track 023: create-scoped VectorCache. Incremental `hnsw_put` stays per-put.

use std::path::Path;

use crate::runtime::hnsw_create_stats::{self, HnswCreateStatsSnapshot};
use crate::runtime::hnsw_fixture::{
    hnsw_create, import_snippet_embeddings, open_sqlite_temp, FIXTURE_DIM, FIXTURE_N, FIXTURE_SEED,
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

fn with_stats<R>(f: impl FnOnce() -> R) -> R {
    hnsw_create_stats::with_exclusive(|| {
        let _guard = StatsEnvGuard;
        enable_create_stats();
        f()
    })
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

/// Process peak working set (Windows) or none on other hosts.
fn peak_rss_bytes() -> Option<u64> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct ProcessMemoryCounters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
        }
        #[link(name = "psapi")]
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn GetProcessMemoryInfo(
                process: *mut std::ffi::c_void,
                ppsmemcounters: *mut ProcessMemoryCounters,
                cb: u32,
            ) -> i32;
        }
        unsafe {
            let mut m = std::mem::zeroed::<ProcessMemoryCounters>();
            m.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
            if GetProcessMemoryInfo(GetCurrentProcess(), &mut m, m.cb) != 0 {
                Some(m.peak_working_set_size as u64)
            } else {
                None
            }
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_create_reuses_one_vector_cache() {
    with_stats(|| {
        let (_tmp, db) = open_sqlite_temp();
        const N: usize = 48;
        const DIM: usize = 8;
        import_snippet_embeddings(&db, N, DIM, FIXTURE_SEED);
        hnsw_create(&db, DIM, 16);
        let snap = hnsw_create_stats::take();
        assert_eq!(
            snap.cache_instances, 1,
            "create must retain one VectorCache, got {snap:?}"
        );
        assert_eq!(
            snap.hnsw_put_count, N as u64,
            "expected one hnsw_put per row, got {snap:?}"
        );
        assert_eq!(
            snap.store_get_count, snap.cache_misses,
            "store_tx.get on ensure_key miss should match cache_misses, got {snap:?}"
        );
        assert!(
            snap.store_get_count < snap.ensure_key_keys,
            "prefetch should avoid a store get per ensure_key, got {snap:?}"
        );
        assert!(
            snap.cache_peak >= N as u64,
            "create-wide cache should hold inserted vectors, peak={}, N={N}, {snap:?}",
            snap.cache_peak
        );
    });
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_incremental_put_still_uses_fresh_cache() {
    hnsw_create_stats::with_exclusive(|| {
        let (_tmp, db) = open_sqlite_temp();
        const DIM: usize = 8;
        import_snippet_embeddings(&db, 16, DIM, FIXTURE_SEED);
        hnsw_create(&db, DIM, 16);

        let _guard = StatsEnvGuard;
        enable_create_stats();
        db.run_default(
            r#"
            ?[id, embedding] <- [[16, vec([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])]]
            :put snippet_embedding {id => embedding}
            "#,
        )
        .unwrap();
        let snap = hnsw_create_stats::take();
        assert_eq!(
            snap.cache_instances, 1,
            "incremental put must allocate a fresh VectorCache, got {snap:?}"
        );
        assert_eq!(
            snap.hnsw_put_count, 1,
            "one incremental hnsw_put, got {snap:?}"
        );
    });
}

#[cfg(feature = "storage-sqlite")]
#[ignore]
#[test]
fn hnsw_prefetch_14k_vs_021() {
    with_stats(|| {
        let rss_before = peak_rss_bytes();
        let (_tmp, db) = open_sqlite_temp();
        import_snippet_embeddings(&db, FIXTURE_N, FIXTURE_DIM, FIXTURE_SEED);
        hnsw_create(&db, FIXTURE_DIM, 100);
        let snap = hnsw_create_stats::take();
        let rss_after = peak_rss_bytes();
        eprintln!("prefetch_ef100 {}", snap.to_json_value());
        if let (Some(before), Some(after)) = (rss_before, rss_after) {
            eprintln!(
                "rss_before_bytes={before} peak_working_set_bytes={after} delta={}",
                after.saturating_sub(before)
            );
        }
        maybe_write_snapshot(&snap, "prefetch_ef100.json");
        assert_eq!(snap.cache_instances, 1);
        assert_eq!(snap.hnsw_put_count, FIXTURE_N as u64);
        assert_eq!(snap.store_get_count, snap.cache_misses);
        assert!(snap.store_get_count < snap.ensure_key_keys);
    });
}

#[cfg(feature = "storage-sqlite")]
#[test]
fn hnsw_prefetch_search_still_works() {
    let (_tmp, db) = open_sqlite_temp();
    import_snippet_embeddings(&db, 32, 8, FIXTURE_SEED);
    hnsw_create(&db, 8, 16);
    let res = db
        .run_default(
            r#"
            ?[dist, id] := ~snippet_embedding:snippet_idx{id | query: q, k: 3, ef: 16, bind_distance: dist}, q = vec([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
            :order dist
            "#,
        )
        .unwrap();
    assert!(!res.rows.is_empty());
}
