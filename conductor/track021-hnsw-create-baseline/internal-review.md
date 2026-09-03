# Track 021 internal review — HNSW create baseline

**Harness:** Cursor Grok (internal, post-implement)
**Track:** `conductor/track021-hnsw-create-baseline`
**Branch:** `track/021-hnsw-create-baseline`
**Date:** 2026-09-02
**Scope:** spec + plan + results + fixture + instrumentation. Read-only except this file.

## Summary

**Clean.** No critical / high / medium findings. Instrumentation is env-gated, batch-timed, and does not change result order or storage format. The 14k Windows release table matches `raw/ef100.json` / `ef50.json`. Keep/kill rows for 022–028 match spec thresholds.

## Findings above low

**None.**

## Hunt checklist

| Hunt | Result |
| :--- | :--- |
| `Instant` inside `par_iter` | **Pass.** Dist timer wraps the whole `distances` collect in `time_dist_batch` (`hnsw.rs` ~852–880). The `par_iter` map has no `Instant`. |
| `unwrap` / `expect` in production | **Pass.** None in `hnsw_create_stats.rs` or the new call sites in `hnsw.rs` / `relation.rs` / `db.rs`. Test-only unwraps in `hnsw_create_stats_test.rs` are fine. Pre-existing `lock.write().unwrap()` in `db.rs` is not this track. |
| `tracing` crate | **Pass.** No `tracing` in workspace `Cargo.toml` or new code. Dump is `eprintln` JSON. |
| Result-order or storage-format change | **Pass.** `hnsw_store_put` is `store_tx.put` plus an env-gated timer. Neighbor walk / heap / encode paths are unchanged. |
| Default-on stats | **Pass.** `ACTIVE` starts false. Hot paths use `is_active()`. `run_sys_op` calls `reset()` / dump only when `CreateVectorIndex` **and** `COZO_HNSW_CREATE_STATS=1\|true`. |
| VectorCache instance counted on search | **Pass.** `record_cache_instance()` is only in `hnsw_put_body` (`hnsw.rs` ~1092–1098). `hnsw_knn` (~1486) and `hnsw_remove` (~1139) construct caches without that counter. |
| Commit mixed into graph remainder | **Pass.** `graph_heaps_ns = create_total − scan − ensure_key − dist − put`. `create_total` is `SessionTx::create_hnsw_index` only. `tx.commit_tx()` is a separate bucket in `run_sys_op` (`db.rs` ~1491–1493). ef100 commit **281 ms** vs create **1.19e6 ms**. |
| 14k fixture shape | **Pass.** SQLite tempfile, N=14000, dim=768 F32, unit-normalized (`norm > 0` then `/=`), seed `21768`, `m: 16`, L2, ef 50 then drop + ef 100. |
| Keep/kill vs thresholds | **Pass.** See matrix below. Numbers match JSON. |
| Smoke too slow / is 14k | **Pass.** Smoke is 32 rows × dim 8 × `ef_construction: 16`. 14k is `#[ignore]`. |
| Placeholders TODO / FIXME | **Pass.** None in new stats files, fixture, results, or the new call sites. |

## DoD matrix

| DoD | Status | Evidence |
| :--- | :--- | :--- |
| Cost table under the track folder | **Pass** | `results.md` + `raw/ef50.json` + `raw/ef100.json` |
| VectorCache lifetime confirmed (per-put vs create-wide) | **Pass** | `cache_instances == 14000 == N`; constructor + count only in `hnsw_put_body` |
| Keep/kill filled for 022–028 | **Pass** | `spec.md` matrix + `results.md` summary. 027 is explicitly **deferred / not KEEP** (train_pq not run). This spec is the authority; orchestrator owns `conductor.md`. |
| Fixture instructions; seeded; Windows numbers | **Pass** | `fixture.md`; seed 21768; host `x86_64-pc-windows-msvc`; `--release` |
| Engine counters: fmt / clippy / nextest / `compact-single-threaded` | **Not re-run here** (read-only). Code path uses `web_time::Instant` + atomics only; rayon is `cfg(feature = "rayon")`. Package name is `cozo` (`-p cozo`), as `fixture.md` says. Spec’s `-p cozo-core` would miss. |
| No silent storage-format change | **Pass** | Timing wrappers only |

## Keep / kill vs spec thresholds (ef100, 14k×768, SQLite, Windows, release)

JSON: dist **10.279%**, ensure_key **34.198%**, put **0.945%**, graph/heaps **54.573%**, mean batch `33477876 / 1607948 ≈ 20.8`, puts/vector `1785790 / 14000 ≈ 128`.

| Track | Spec gate | 021 call | Justified? |
| :--- | :--- | :--- | :--- |
| **022** SIMD L2 | KEEP if dist ≥ **25%** or alloc-free L2 ≥2× microbench. KILL if put+ensure_key ≫ dist. | **KILL** — dist 10.3% < 25%; ensure_key+put **35.1%** ≫ dist. No microbench. | **Yes.** Kill clause fires. A 2× L2 microbench would not move create (dist is 10%). |
| **023** prefetch / decode | KEEP if ensure_key/decode ≥ **25%**. KILL if distance math dominates. | **KEEP** — ensure_key **34.2%**. | **Yes.** |
| **024** put batching | KEEP if put ≥ **30%** **or** put count huge vs graph CPU. | **KILL** — put **0.94%** ≪ 30%; ~128 puts/vector but put wall ≪ graph **54.6%**. | **Yes.** The “or count” clause is vs **graph CPU time**, not raw statement count. 11 s of put vs ~20 min create is not the bottleneck. |
| **025** incremental vs rebuild | KEEP if drop+create is the pain. KILL if create is already cheap vs ingest. | **KEEP** — ~20 min create. | **Yes.** |
| **026** knobs | Always KEEP; share fixture. | **KEEP**. | **Yes.** |
| **027** PQ | KEEP only after 021; KILL if `train_pq` ≥ L2 create. | **deferred / not KEEP** — `train_pq` not run. | **Yes.** Cannot KEEP or KILL without a train_pq number. Leave 027 as its own spike. |
| **028** GPU dist | KEEP only if 021+022 leave a large dist gap **and** batches amortize GPU copy. KILL if batches are ~m-sized. | **KILL** — dist 10.3%; mean batch **~20.8** (m=16, m_max0=32). | **Yes.** Not ≫ m; dist does not dominate. |

Largest measured bucket is **graph/heaps 54.6%**, which 022–028 do not own. `results.md` already flags that for later planning. Not a 021 defect.

## Lows (do not block)

1. **`COZO_HNSW_CREATE_STATS` is process-global.** Smoke sets it via `set_var` in the lib test binary that also has many `::hnsw create` tests. A parallel nextest neighbor could `reset()` counters before `take()`. Loose `> 0` asserts and a `Drop` unsetter make a flake unlikely; 14k is `#[ignore]`.
2. **Failed `CreateVectorIndex` skips `dump_stderr`**, so `ACTIVE` can stay true for the rest of that process if the env flag is on. Default-off is unchanged.
3. **`ensure_key` / dist timers cover `hnsw_search_level` neighbor batches only** (as spec required). Entry-point `ensure_key`/`v_dist` and heuristic distances land in graph remainder. That **understates** dist/ensure slightly and does not flip 022/023.
4. **Empty `unvisited` batches still increment batch counters.** Mean 20.8 is a lower bound on non-empty batch size; still not GPU-sized.
5. **No compact-single-threaded log in `results.md`.** Command in `fixture.md` is the correct `-p cozo` form.

## What looks solid

- Scan Instant is `is_active().then(Instant::now)` — no timer when off.
- Put helper is the single HNSW `store_tx.put` path; metadata put at the end of create goes through it (one extra counted put; negligible vs 1.79M).
- Intra-put hit/miss is counted in `ensure_key`; ef100 hit rate ~92.6% with `cache_instances == N` proves per-put, not create-wide.
- `%` in `results.md` match JSON (rounding only).
- Smoke cannot accidentally run 14k.

## Research / tools notes

- ledgerful: `doctor --json` readyForPublish true (unrelated warns: graph content-stale, sig pin). `ledger status --compact`: 1 pending (not touched).
- ai-brains: `preflight --summary` from `C:\dev\CozoDB-redux`.
- cargo 1.95.0. No `scan --impact` / `change-context` (read-only).

## Verdict

**Internal review clean.** No open finding above **low**. Keep/kill authority in `spec.md` is justified by the measured table.
