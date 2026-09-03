# Track Completion Audit — track021-hnsw-create-baseline

## Verdict: PASS

Core measurement DoD is met. No P0–P2. Three non-blocking P3s below; none change 022–028 keep/kill.

## Scope Reviewed

- Track: `C:\dev\CozoDB-redux\conductor\track021-hnsw-create-baseline`
- Execution repo: `C:\dev\CozoDB-redux`
- Branch: `track/021-hnsw-create-baseline` vs `origin/main`
- HEAD: `307ae144` (Point published fork URLs at Ryan-AI-Studios/cozo-redux after the GitHub move)
- Included: committed + unstaged + untracked working tree (read-only except this file)

Working-tree vs `origin/main` (product):

| Path | Role |
| :--- | :--- |
| `cozo-core/src/runtime/hnsw_create_stats.rs` | new env-gated counters |
| `cozo-core/src/runtime/hnsw_create_stats_test.rs` | new smoke + ignored 14k fixture |
| `cozo-core/src/runtime/mod.rs` | module wiring |
| `cozo-core/src/runtime/hnsw.rs` | batch timers, `hnsw_store_put`, per-put cache count |
| `cozo-core/src/runtime/relation.rs` | `create_hnsw_index` scan + create_total wrap |
| `cozo-core/src/runtime/db.rs` | CreateVectorIndex reset / commit bucket / stderr dump |

Track artifacts: `spec.md`, `plan.md`, `results.md`, `fixture.md`, `raw/ef50.json`, `raw/ef100.json`, `internal-review.md`. `conductor/conductor.md` and `docs/deferred.md` diffs are registry/planning notes, not engine behavior.

Not modified (confirmed by diff): `parse/sys.rs`, `storage/sqlite.rs`, `BREAKING.md`, `Cargo.toml` (no `tracing`).

This reviewer did **not** re-run cargo fmt/clippy/nextest/`compact-single-threaded` or the ignored 14k create. Those are cited as orchestrator-reported except where the published JSON can be checked by reading.

## Requirement and DoD Matrix

| Requirement | Status | Evidence | Tests | Gap |
| :--- | :--- | :--- | :--- | :--- |
| 1. Repeatable Ledgerful-shaped fixture: SQLite, F32, dim 768, ~14k, `m: 16`, L2, drop+create, ef 50 and 100, seeded unit-normalized | **Met** | `hnsw_create_stats_test.rs`: `FIXTURE_N=14000`, `FIXTURE_DIM=768`, seed `21768`, `DbInstance::new("sqlite", …)`, L2-normalize then import; create script `m: 16`, ef 50 then drop then 100 | Smoke: 32×8, not ignored. 14k: `#[ignore]`. JSON `cache_instances=14000` | None material |
| 2. Primary numbers on Windows | **Met** | `results.md` host `x86_64-pc-windows-msvc`; `--release` | 14k run is the evidence | No Unix column (optional) |
| 3a. `scan_all` / TempCollector bucket | **Met** | `relation.rs` Instant around `scan_all` → `TempCollector`; ef100 `scan_ms=67.0` (0.01%) | Smoke does not assert scan | — |
| 3b. `ensure_key` timed as whole neighbor **batch**, not per key | **Met** | `hnsw.rs` `time_ensure_batch(unvisited.len(), …)` around the sequential loop; ef100 1,607,948 batches / 33,477,876 keys | Smoke: `cache_instances > 0` only | Entry-point / heuristic `ensure_key` are outside the batch timer by spec (remainder) |
| 3c. Intra-put VectorCache hit/miss + instance count | **Met** | hit/miss in `VectorCache::ensure_key`; `record_cache_instance` only in `hnsw_put_body` after a non-empty extract; search (`~1486`) and remove (`~1139`) constructors are not counted. ef100: 14,000 instances, 415,490,175 hits / 33,130,117 misses (92.6% hits) | Smoke asserts instances > 0; 14k asserts both creates | — |
| 3d. `dist` / `v_dist` at **batch** level; no `Instant` inside `par_iter` | **Met** | `time_dist_batch` wraps the whole `distances` collect; `par_iter().map(pq_compute)` has no `Instant`. Sequential path when `unvisited.len() < 8` or `not(rayon)` | Smoke: `dist_ns > 0 \|\| dist_batches > 0` | Heuristic `k_dist` is remainder (spec scoped dist to `hnsw_search_level`) |
| 3e. graph/heaps remainder **without** `tx.commit()` / `FlushFileBuffers` | **Met** | `graph_heaps_ns = create_total − scan − ensure_key − dist − put`; `create_total` is `SessionTx::create_hnsw_index` only. Commit is a separate bucket in `run_sys_op`. ef100 commit 281 ms vs create 1,191,856 ms | JSON fields `commit_ms` vs `graph_heaps_ms` | Remainder also includes heuristic L2, shrink, encode, neighbor walk (labeled in `results.md`) |
| 3f. `store_tx.put` count + time | **Partial** | Most HNSW puts go through `hnsw_store_put` → `time_put`. ef100 `put_count=1,785,790` (~128/vector), `put_pct=0.94%` | Smoke/14k assert `put_count > 0` | One create-path self-loop `store_tx.put` is not wrapped (P3). Cannot flip 024 |
| 3g. `tx.commit()` own bucket **or** bound table to `create_hnsw_index` | **Met** | Both: table `%` is of `create_total`; commit is timed at `Db::run_sys_op` | JSON `commit_ms` present on both creates | — |
| 4. Env-gated Instant; default off; no tracing; no format/order change; no new production `unwrap`/`expect` | **Met** | `ACTIVE` starts false; hot path is `is_active()`; `reset()`/`dump_stderr` only when `SysOp::CreateVectorIndex` **and** env `1`/`true`; dump is `eprintln` JSON; no `tracing` dep; `hnsw_store_put` is put + timer | Tests `set_var` then `reset()` | Process-global flag (P3) |
| 5. Written keep/kill for 022–028; this spec is the authority | **Met** | `spec.md` matrix filled from ef100; `results.md` summary matches JSON and thresholds | N/A | 027 is explicit **deferred / not KEEP** (`train_pq` not run; PQ is out of scope). `conductor.md` not annotated (P3) |
| 6. `compact-single-threaded` still builds if engine counters land | **Not verifiable here** (orchestrator-reported pass) | Stats module is atomics + `web_time::Instant` only; dist collect has `#[cfg(not(feature = "rayon"))]`. Package name is `cozo`, not `cozo-core` (`fixture.md` has the correct `-p cozo`) | — | Spec’s `-p cozo-core` command would not match a package |
| DoD: cost table in track folder | **Met** | `results.md` + `raw/ef50.json` + `raw/ef100.json`; published ms/% match JSON rounding | — | Folder is gitignored; files exist in the working tree |
| DoD: VectorCache lifetime in the table (per-put vs create-wide) | **Met** | `cache_instances == N` on both creates; `results.md` states per-`hnsw_put` | — | — |
| DoD: keep/kill rows; conductor notes if killed | **Partial** | Spec matrix filled (authority). 022/024/028 **KILL**, 023/025/026 **KEEP**, 027 deferred | — | `conductor.md` still lists 022/024/028 as Ready with “After 021 keep” on 022 (P3) |
| DoD: fixture script/instructions; seeded; Windows numbers | **Met** | `fixture.md` PowerShell; seed 21768; Windows release table | Smoke in `nextest --lib`; 14k ignored | — |
| DoD: fmt, clippy `-D warnings`, `nextest --lib --bins --workspace` | **Not verifiable here** | Orchestrator: clippy pass; nextest 183 passed, 1 skipped (ignored 14k) | Matches `#[ignore]` on `hnsw_create_baseline_14k` | Not re-run |
| DoD: no silent storage-format change | **Met** | Timing wrappers only; manifest / encode unchanged | — | — |
| Hard lock: `::hnsw create { dim, dtype: F32, fields, distance: L2, m, ef_construction }` | **Met** | Fixture uses that sysop; parser not in the diff | Smoke + 14k | — |
| Hard lock: `cozo-core` owns engine; no FAISS/Qdrant | **Met** | Counters stay in `cozo-core` | — | — |
| Hard lock: miette + `Result`; no production unwrap/expect in new code | **Met** | New stats helpers return `Result`/`T`; no unwrap in `hnsw_create_stats.rs` or new call sites. Test-only unwraps. Pre-existing `lock.write().unwrap()` in `db.rs` CreateVectorIndex is unchanged | — | — |

Keep/kill vs stated thresholds (ef100, 14k×768, SQLite, Windows, release). JSON: dist **10.279%**, ensure_key **34.198%**, put **0.945%**, graph/heaps **54.573%**, ensure+put **35.14%**, mean batch `33477876/1607948 ≈ 20.8`, puts/vector `1785790/14000 ≈ 127.6`.

| Track | Gate | 021 call | Justified |
| :--- | :--- | :--- | :--- |
| **022** | KEEP if dist ≥ 25% or alloc-free L2 ≥2× microbench. KILL if put+ensure_key ≫ dist | **KILL** | dist 10.3% < 25%; 35.1% ≫ 10.3%. A 2× L2 microbench cannot move create |
| **023** | KEEP if ensure_key ≥ 25% | **KEEP** | 34.2% |
| **024** | KEEP if put ≥ 30% **or** put count huge **vs graph CPU**. KILL if wall is not the bottleneck | **KILL** | 0.94% ≪ 30%; ~128 puts/vector but 11 s put vs ~20 min create / 54.6% graph. Reading “vs graph CPU” as wall time is correct |
| **025** | KEEP if drop+create is product pain | **KEEP** | ~19.9 min create |
| **026** | Always KEEP | **KEEP** | Independent; fixture shared |
| **027** | KILL if `train_pq` ≥ L2 create; otherwise later spike | **deferred / not KEEP** | `train_pq` not run (out of scope; not cheap vs this 20 min create). Cannot KEEP or KILL |
| **028** | KEEP only if dist still dominates **and** batches amortize GPU. KILL if batches ~m-sized | **KILL** | dist 10.3%; mean batch ~20.8 (m=16, m_max0=32) |

Largest bucket is graph/heaps **54.6%**, which 022–028 do not own. `results.md` already flags that.

## Findings

### [P3] Self-loop `store_tx.put` is not routed through `hnsw_store_put`

Confidence: High

Requirement: Wall-clock `store_tx.put` count + time on the create path

Location: `cozo-core\src\runtime\hnsw.rs:487-488` (`hnsw_put_vector`, “add self-link”)

Problem: The instrumentation pass replaced HNSW `store_tx.put` with `hnsw_store_put` except this split-line call. Each inserted vector still writes a per-level self-loop record via raw `put`, so those statements are not in `put_count` / `put_ns` and land in graph/heaps remainder.

Evidence: Grep of `.put(` in `hnsw.rs` is only the wrapper at `:348` and this call at `:488`. Neighbor / canary / shrink / PQ puts use `hnsw_store_put`. For levels `max(target, bottom)..=0` this is typically ~1 put/vector → ~14k vs ef100 `put_count=1,785,790` (~0.8% of puts). Put wall is 0.94% of create.

Failure scenario: A later reader treats `put_pct=0.94%` as every create-path put. The undercount is far below the 024 30% gate (~128 → ~129 puts/vector still “count huge, wall tiny”).

Correction: Route this call through `hnsw_store_put`. Do not re-run 14k for 021 keep/kill. Optionally note in `results.md` that self-loop puts were in remainder.

Verification: Search `hnsw.rs` for `store_tx.put`; only `hnsw_store_put` should remain. Smoke still `put_count > 0`.

Deferrable: Yes — does not change 022–028.

### [P3] `conductor.md` is not updated for 021 kill results

Confidence: High

Requirement: DoD — keep/kill filled for 022–028; conductor notes updated if a track is killed at measurement time (`spec.md` remains the keep/kill authority)

Location: `conductor\conductor.md:9-23`

Problem: Spec matrix KILLs 022, 024, and 028. The registry still lists them Ready — not started. Suggested order still runs 022 after 026. 022’s blurb still says “After **021** keep.”

Evidence: Diff vs `origin/main` upgrades Placeholder → Ready / 021 In progress; no KILL annotations. `spec.md` and `results.md` are filled.

Failure scenario: A later implementer starts 022 from the registry without reading the 021 matrix.

Correction: Annotate 022/024/028 as killed-at-021 (point at this spec). Keep 023/025/026 KEEP; leave 027 as its own spike. Orchestrator-owned wrap-up.

Verification: Registry text matches the spec matrix.

Deferrable: No — one-line registry edit; do it before marking 021 complete. Not a product defect.

### [P3] Create-stats env flag is process-global

Confidence: Medium

Requirement: Env-gated counters; default off; tests must not disturb other `::hnsw create` work

Location: `cozo-core\src\runtime\hnsw_create_stats.rs:22` (`ACTIVE`); `hnsw_create_stats_test.rs:24-37`; `db.rs:1480-1493`

Problem: `COZO_HNSW_CREATE_STATS` and the cached `ACTIVE` bit are process-wide. Smoke calls `set_var("1")` then `reset()` inside the lib test binary. Parallel nextest workers sharing that process can `reset()` or `dump_stderr()` around another test’s `::hnsw create`. Default-off for production is unchanged (`ACTIVE` starts false; `run_sys_op` only arms on CreateVectorIndex when the env is on).

Evidence: `enable_create_stats` uses process env; `dump_stderr` stores `ACTIVE=false` after one create. Orchestrator reported 183 passed / 1 skipped — no observed flake in this review.

Failure scenario: Parallel lib test sets/clears the flag while smoke is between create and `take()`, so `put_count == 0` and smoke fails, or another create is timed.

Correction: Prefer a test-only arming path (or serial `#[serial]` for the smoke), not `set_var` for the whole process. Production can keep getenv + `reset()` on CreateVectorIndex.

Verification: `nextest --lib --bins --workspace` with default parallelism; smoke still passes.

Deferrable: Yes — no observed flake; production default-off holds.

## Completeness Sweep

Searched new/changed runtime files for `TODO` / `FIXME` / `XXX` / `HACK` / `placeholder` / `unimplemented!`: **none**.

No false-success: smoke and 14k assert `put_count > 0`, cache instances, and some dist work. 14k is `#[ignore]` on purpose (fixture.md); nextest skip count of 1 matches.

No SIMD/GPU/PQ feature-gated dead code claimed as shipped. This track is measurement only.

`take()` is `#[allow(dead_code)]` for non-test builds; tests use it. Production dump is `dump_stderr()`.

`write_idx_relation` schema puts in `relation.rs` remain untimed `store_tx.put` (a handful per create). Same class as the self-loop miss; cannot move put share to 30%.

Spec compact-check command uses `-p cozo-core`; crate package name is `cozo`. `fixture.md` is correct. Implementation still compiles without rayon (code inspection). Orchestrator reported the check passed.

`docs/deferred.md` diff is planning text for D-009-01 / D-012-01, not a 021 measurement hole.

## Wiring and Regression Review

```
SysOp::CreateVectorIndex
  → Db::run_sys_op
      if env on: hnsw_create_stats::reset()
      transact_write
      → run_sys_op_with_tx → SessionTx::create_hnsw_index
          record_create_total
            scan_all + TempCollector          → scan_ns
            sequential hnsw_put per tuple
              record_hnsw_put
                VectorCache in hnsw_put_body  → cache_instances (per put)
                hnsw_put_vector
                  ensure_key batch            → ensure_key_ns (search_level only)
                  dist batch (± rayon)        → dist_ns (around distances Vec)
                  hnsw_store_put              → put_ns / put_count
          (graph/heaps = create_total − those)
      record_commit(tx.commit_tx)             → commit_ns (not in remainder)
      dump_stderr JSON; ACTIVE=false
```

Default off: `is_active()` is a relaxed atomic; Instant is not taken on scan/ensure/dist/put/commit when false. `enabled()` getenv runs only at CreateVectorIndex in `run_sys_op`.

`compact-single-threaded` = `minimal + requests + graph-algo` without `rayon`. Dist collect has a `not(rayon)` branch. Stats module has no rayon.

Determinism / Datalog: no planner or result-order change. Storage backends: SQLite fixture only; create still uses the existing write txn (`SqliteTx::put` → `par_put` unchanged). Windows paths: tempfile + optional `COZO_HNSW_CREATE_STATS_OUT`.

`HNSW_PAR_DIST_THRESHOLD` remains 8. Create still passes `filter: None`, `pq_dist_table: None`. Parser defaults for `m` / `ef_construction` unchanged (not in the diff).

Focus checks:

| Focus | Result |
| :--- | :--- |
| Instant inside `par_iter` | **Pass.** Timer wraps the collect; map closure is `v_dist` / PQ only |
| unwrap in production (new code) | **Pass** |
| stats default-off | **Pass.** `ACTIVE=false` until reset after env on |
| commit mixed into remainder | **Pass.** Separate bucket; 281 ms vs 19.9 min |
| keep/kill vs thresholds | **Pass.** Arithmetic matches JSON |
| VectorCache per-put | **Pass.** 14,000 instances == N; not create-wide |
| compact-single-threaded | **Code-compatible; check reported, not re-run** |
| fixture unit-norm SQLite 768 14k m:16 | **Pass** |

## Verification Evidence

**Observed now (read):**

- `raw/ef50.json` and `raw/ef100.json` match `results.md` (ms/%, counts, cache).
- ef100: scan 67.0 ms, ensure 407,587 (34.2%), dist 122,515 (10.3%), put 11,259 (0.94%), graph 650,428 (54.6%), create 1,191,856, commit 281, instances 14,000, puts 1,785,790.
- Bucket sum: scan + ensure + dist + put + graph = create_total (saturating remainder formula).
- Mean batch 20.8; puts/vector ~128.
- `hnsw_create_stats.rs` has no `unwrap`/`expect`; `ACTIVE` default false; no `Instant` in the rayon map.
- Git diff vs `origin/main` is counters + tests + docs; no parser/SQLite/format change.

**Reported by orchestrator (not re-run):**

- `cargo clippy --all-targets --all-features -- -D warnings` pass
- `cargo nextest run --lib --bins --workspace` — 183 passed, 1 skipped
- `cargo check -p cozo --no-default-features --features compact-single-threaded` pass
- Ignored 14k in `--release` (~20 min ef100); debug 14k aborted ~29 min
- Internal review: no findings above low

**Recommended (not required to accept 021):** wrap the self-loop `put`; annotate conductor kills; optionally isolate the smoke env flag.

**Not verifiable here:** fmt/clippy/nextest/compact commands themselves; live 14k wall clock; Unix column.

Onboarding (read-only): cargo 1.95.0; `ledgerful doctor --json` readyForPublish true (unrelated warns: graph content-stale, sig pin); ledger `1 pending`; `ai-brains preflight --summary` from `C:\dev\CozoDB-redux`. No `scan --impact`.

## Deferred Candidates

Reviewer proposes only; orchestrator edits `docs/deferred.md`.

| Item | Defer? | Why |
| :--- | :--- | :--- |
| Self-loop `put` not timed | Optional | Easy one-line wrap; does not change keep/kill. Prefer wrap in wrap-up **or** a one-line `results.md` note. Not worth a 14k rerun |
| Process-global stats env | Yes, if not fixed now | Test isolation only; production default-off holds |
| `conductor.md` kill notes | **No** | Easy registry edit before Complete |
| 027 `train_pq` vs L2 create | Already owned by **027** | 021 correctly did not KEEP. Do not close D-012-01 |
| Graph/heaps 54.6% unowned | Future planning, not 021 | No 022–028 spike targets it. Call out after 023 |

Do not defer P0–P2 (none). Easy P3s: conductor notes now; self-loop wrap optional.

## Completion Decision

**PASS.** Track 021 did what it claimed: a Windows release cost model of `::hnsw create` on a Ledgerful-shaped SQLite 14k×768 unit-norm fixture, with env-gated Instant buckets, commit kept out of graph remainder, VectorCache shown per-put (`cache_instances == N`), and keep/kill filled against the stated thresholds.

022/024/028 KILL and 023/025/026 KEEP are justified by the published JSON. 027 stays a later spike because `train_pq` was not run.

Before marking the registry Complete: annotate killed tracks in `conductor.md`. Optional: route the self-loop `put` through `hnsw_store_put` without a new 14k run. Re-review is not required unless a follow-up changes published percentages enough to flip a gate (the known miss cannot).
