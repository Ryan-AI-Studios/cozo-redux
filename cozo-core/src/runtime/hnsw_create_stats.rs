/*
 * Copyright 2023, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Env-gated wall-clock buckets for `::hnsw create`. Default off.
//!
//! Enable with `COZO_HNSW_CREATE_STATS=1` or `true` (case-insensitive), then call
//! [`reset`] so hot paths read a cached [`AtomicBool`] instead of `getenv`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use miette::Result;
use web_time::Instant;

const ENV_NAME: &str = "COZO_HNSW_CREATE_STATS";

static ACTIVE: AtomicBool = AtomicBool::new(false);

static SCAN_NS: AtomicU64 = AtomicU64::new(0);
static ENSURE_KEY_NS: AtomicU64 = AtomicU64::new(0);
static ENSURE_KEY_BATCHES: AtomicU64 = AtomicU64::new(0);
static ENSURE_KEY_KEYS: AtomicU64 = AtomicU64::new(0);
static DIST_NS: AtomicU64 = AtomicU64::new(0);
static DIST_BATCHES: AtomicU64 = AtomicU64::new(0);
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static CACHE_INSTANCES: AtomicU64 = AtomicU64::new(0);
static PUT_NS: AtomicU64 = AtomicU64::new(0);
static PUT_COUNT: AtomicU64 = AtomicU64::new(0);
static HNSW_PUT_NS: AtomicU64 = AtomicU64::new(0);
static HNSW_PUT_COUNT: AtomicU64 = AtomicU64::new(0);
static CREATE_TOTAL_NS: AtomicU64 = AtomicU64::new(0);
static COMMIT_NS: AtomicU64 = AtomicU64::new(0);

fn env_flag_on() -> bool {
    match std::env::var(ENV_NAME) {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        }
        Err(_) => false,
    }
}

fn as_ns(d: Duration) -> u64 {
    d.as_nanos() as u64
}

fn add_ns(slot: &AtomicU64, d: Duration) {
    slot.fetch_add(as_ns(d), Ordering::Relaxed);
}

fn zero_counters() {
    SCAN_NS.store(0, Ordering::Relaxed);
    ENSURE_KEY_NS.store(0, Ordering::Relaxed);
    ENSURE_KEY_BATCHES.store(0, Ordering::Relaxed);
    ENSURE_KEY_KEYS.store(0, Ordering::Relaxed);
    DIST_NS.store(0, Ordering::Relaxed);
    DIST_BATCHES.store(0, Ordering::Relaxed);
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
    CACHE_INSTANCES.store(0, Ordering::Relaxed);
    PUT_NS.store(0, Ordering::Relaxed);
    PUT_COUNT.store(0, Ordering::Relaxed);
    HNSW_PUT_NS.store(0, Ordering::Relaxed);
    HNSW_PUT_COUNT.store(0, Ordering::Relaxed);
    CREATE_TOTAL_NS.store(0, Ordering::Relaxed);
    COMMIT_NS.store(0, Ordering::Relaxed);
}

/// Read `COZO_HNSW_CREATE_STATS` (does not cache). Prefer [`is_active`] on hot paths.
pub(crate) fn enabled() -> bool {
    env_flag_on()
}

/// Re-read the env flag into the cached active bit and zero all counters.
/// Tests must call this **after** setting the env var.
pub(crate) fn reset() {
    ACTIVE.store(env_flag_on(), Ordering::Release);
    zero_counters();
}

/// Cached flag. Hot paths must use this instead of `getenv`.
pub(crate) fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub(crate) fn record_scan(d: Duration) {
    if is_active() {
        add_ns(&SCAN_NS, d);
    }
}

pub(crate) fn time_ensure_batch<T>(n_keys: usize, f: impl FnOnce() -> Result<T>) -> Result<T> {
    if !is_active() {
        return f();
    }
    ENSURE_KEY_BATCHES.fetch_add(1, Ordering::Relaxed);
    ENSURE_KEY_KEYS.fetch_add(n_keys as u64, Ordering::Relaxed);
    let start = Instant::now();
    let out = f();
    add_ns(&ENSURE_KEY_NS, start.elapsed());
    out
}

pub(crate) fn record_cache_hit() {
    if is_active() {
        CACHE_HITS.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn record_cache_miss() {
    if is_active() {
        CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn record_cache_instance() {
    if is_active() {
        CACHE_INSTANCES.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn time_dist_batch<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    if !is_active() {
        return f();
    }
    DIST_BATCHES.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    let out = f();
    add_ns(&DIST_NS, start.elapsed());
    out
}

pub(crate) fn time_put<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    if !is_active() {
        return f();
    }
    PUT_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    let out = f();
    add_ns(&PUT_NS, start.elapsed());
    out
}

pub(crate) fn record_hnsw_put<T>(f: impl FnOnce() -> T) -> T {
    if !is_active() {
        return f();
    }
    HNSW_PUT_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    let out = f();
    add_ns(&HNSW_PUT_NS, start.elapsed());
    out
}

pub(crate) fn record_create_total<T>(f: impl FnOnce() -> T) -> T {
    if !is_active() {
        return f();
    }
    let start = Instant::now();
    let out = f();
    add_ns(&CREATE_TOTAL_NS, start.elapsed());
    out
}

pub(crate) fn record_commit<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    if !is_active() {
        return f();
    }
    let start = Instant::now();
    let out = f();
    add_ns(&COMMIT_NS, start.elapsed());
    out
}

/// Counters collected while stats are active. Times are nanoseconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HnswCreateStatsSnapshot {
    pub scan_ns: u64,
    pub ensure_key_ns: u64,
    pub ensure_key_batches: u64,
    pub ensure_key_keys: u64,
    pub dist_ns: u64,
    pub dist_batches: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_instances: u64,
    pub put_ns: u64,
    pub put_count: u64,
    pub hnsw_put_ns: u64,
    pub hnsw_put_count: u64,
    pub create_total_ns: u64,
    pub commit_ns: u64,
    /// `create_total − scan − ensure_key − dist − put`. Does **not** include commit.
    pub graph_heaps_ns: u64,
}

impl HnswCreateStatsSnapshot {
    fn from_atomics() -> Self {
        let scan_ns = SCAN_NS.load(Ordering::Relaxed);
        let ensure_key_ns = ENSURE_KEY_NS.load(Ordering::Relaxed);
        let dist_ns = DIST_NS.load(Ordering::Relaxed);
        let put_ns = PUT_NS.load(Ordering::Relaxed);
        let create_total_ns = CREATE_TOTAL_NS.load(Ordering::Relaxed);
        let graph_heaps_ns = create_total_ns
            .saturating_sub(scan_ns)
            .saturating_sub(ensure_key_ns)
            .saturating_sub(dist_ns)
            .saturating_sub(put_ns);
        Self {
            scan_ns,
            ensure_key_ns,
            ensure_key_batches: ENSURE_KEY_BATCHES.load(Ordering::Relaxed),
            ensure_key_keys: ENSURE_KEY_KEYS.load(Ordering::Relaxed),
            dist_ns,
            dist_batches: DIST_BATCHES.load(Ordering::Relaxed),
            cache_hits: CACHE_HITS.load(Ordering::Relaxed),
            cache_misses: CACHE_MISSES.load(Ordering::Relaxed),
            cache_instances: CACHE_INSTANCES.load(Ordering::Relaxed),
            put_ns,
            put_count: PUT_COUNT.load(Ordering::Relaxed),
            hnsw_put_ns: HNSW_PUT_NS.load(Ordering::Relaxed),
            hnsw_put_count: HNSW_PUT_COUNT.load(Ordering::Relaxed),
            create_total_ns,
            commit_ns: COMMIT_NS.load(Ordering::Relaxed),
            graph_heaps_ns,
        }
    }

    fn pct(part: u64, total: u64) -> f64 {
        if total == 0 {
            0.0
        } else {
            100.0 * part as f64 / total as f64
        }
    }

    fn ms(ns: u64) -> f64 {
        ns as f64 / 1_000_000.0
    }

    pub(crate) fn to_json_value(&self) -> serde_json::Value {
        let total = self.create_total_ns;
        serde_json::json!({
            "scan_ns": self.scan_ns,
            "scan_ms": Self::ms(self.scan_ns),
            "scan_pct": Self::pct(self.scan_ns, total),
            "ensure_key_ns": self.ensure_key_ns,
            "ensure_key_ms": Self::ms(self.ensure_key_ns),
            "ensure_key_pct": Self::pct(self.ensure_key_ns, total),
            "ensure_key_batches": self.ensure_key_batches,
            "ensure_key_keys": self.ensure_key_keys,
            "dist_ns": self.dist_ns,
            "dist_ms": Self::ms(self.dist_ns),
            "dist_pct": Self::pct(self.dist_ns, total),
            "dist_batches": self.dist_batches,
            "cache_hits": self.cache_hits,
            "cache_misses": self.cache_misses,
            "cache_instances": self.cache_instances,
            "put_ns": self.put_ns,
            "put_ms": Self::ms(self.put_ns),
            "put_pct": Self::pct(self.put_ns, total),
            "put_count": self.put_count,
            "hnsw_put_ns": self.hnsw_put_ns,
            "hnsw_put_ms": Self::ms(self.hnsw_put_ns),
            "hnsw_put_count": self.hnsw_put_count,
            "graph_heaps_ns": self.graph_heaps_ns,
            "graph_heaps_ms": Self::ms(self.graph_heaps_ns),
            "graph_heaps_pct": Self::pct(self.graph_heaps_ns, total),
            "create_total_ns": self.create_total_ns,
            "create_total_ms": Self::ms(self.create_total_ns),
            "commit_ns": self.commit_ns,
            "commit_ms": Self::ms(self.commit_ns),
        })
    }
}

pub(crate) fn snapshot() -> HnswCreateStatsSnapshot {
    HnswCreateStatsSnapshot::from_atomics()
}

/// Snapshot then zero counters (does not change the cached active flag).
#[allow(dead_code)] // used by `#[cfg(test)]` harnesses; keep in the pub(crate) API
pub(crate) fn take() -> HnswCreateStatsSnapshot {
    let snap = snapshot();
    zero_counters();
    snap
}

/// One JSON line on stderr, then deactivate so later search/put traffic is not timed.
pub(crate) fn dump_stderr() {
    if !is_active() {
        return;
    }
    let snap = snapshot();
    eprintln!("{}", snap.to_json_value());
    ACTIVE.store(false, Ordering::Release);
}
