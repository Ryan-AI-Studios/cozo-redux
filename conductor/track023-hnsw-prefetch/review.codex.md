# Track Completion Audit — track023-hnsw-prefetch
## Verdict: PASS WITH DEFERRED P3

## Scope Reviewed
Reviewed the `track/023-hnsw-prefetch` working tree against `origin/main` in `C:\dev\CozoDB-redux`, including the runtime/code changes, track docs, and ignored release artifact `conductor/track023-hnsw-prefetch/raw/prefetch_ef100.json`. I also checked the untracked test file [hnsw_prefetch_test.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_prefetch_test.rs:1>).

`ledgerful` and `ai-brains` were not usable in this environment: `ledgerful doctor/status` failed with `unable to open database file`, and `ai-brains preflight` failed with `VAULT_KEY_MISSING`. Cargo gates were not rerun in this read-only review.

## Requirement and DoD Matrix
| Item | Status | Evidence | Tests | Gap |
|---|---|---|---|---|
| Req 1: create-only scoped cache; incremental path untouched; no format change | Met | [relation.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1209>) creates one `VectorCache` and calls [hnsw_put_with_cache](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1080>) only from create; incremental path in [stored.rs](</C:/dev/CozoDB-redux/cozo-core/src/query/stored.rs:434>) still calls `hnsw_put` | [hnsw_prefetch_test.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_prefetch_test.rs:141>) | None |
| Req 2: document RSS; cap can defer for spike | Partial | RSS and scaling are documented in [results.md](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/results.md:30>); deferral is recorded in [deferred.md](</C:/dev/CozoDB-redux/docs/deferred.md:31>) | Ignored release test in [hnsw_prefetch_test.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_prefetch_test.rs:172>) plus raw JSON | No RAM cap yet |
| Req 3: one write txn; no torn index; lock documented | Met | `CreateVectorIndex` still runs under `lock.write()` in [db.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/db.rs:1288>) and is documented in [results.md](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/results.md:48>) | Code inspection | None |
| Req 4: win metric is `store_tx.get` count/ns | Met | `ensure_key` wraps `handle.get` with [time_store_get](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:285>), and stats expose `store_get_count/ns` in [hnsw_create_stats.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_create_stats.rs:177>) | Raw [prefetch_ef100.json](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/raw/prefetch_ef100.json:1>) shows `store_get_count: 0` | None |
| Req 5: no pre-`ensure_key` reread of scanned tuples | Met | Create loop just reuses a retained cache from [relation.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1209>); self-warming comes from [vec_cache.insert](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:374>) | [hnsw_prefetch_test.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_prefetch_test.rs:107>) and raw JSON | None |
| Req 6: do not retune `HNSW_PAR_DIST_THRESHOLD` here | Met | Threshold remains `8` in [hnsw.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:32>) | Code inspection | None |
| Req 7: `compact-single-threaded` unchanged | Not verifiable | No compact-specific code path was changed | Reported by orchestrator: `cargo check -p cozo --no-default-features --features compact-single-threaded` passed | I did not rerun it |
| DoD: 14k create-time improvement vs 021 | Met | [results.md](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/results.md:11>) records `19.9 min -> 12.6 min`; raw JSON records `create_total_ms`, `store_get_count=0`, `cache_instances=1` | Ignored release test output | None |
| DoD: keep/kill recorded | Met | [results.md](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/results.md:7>) and [spec.md](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/spec.md:76>) record `KEEP` | Docs | None |
| DoD: no format break without `BREAKING.md` | Met | I saw no storage layout or migration change in the code path; `BREAKING.md` is unchanged in the diff | Code inspection | None |

## Findings
[P3] Default create-wide `VectorCache` still has no RAM cap
Confidence: High
Requirement: Req 2 / T023-F01
Location: [relation.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1209>), [results.md](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/results.md:30>), [deferred.md](</C:/dev/CozoDB-redux/docs/deferred.md:31>)
Problem: The default create path now retains all vectors for the duration of create, and there is still no cap or bound.
Evidence: The code holds one cache across the full create loop; the measured 14k×768 run reports peak working set about 119 MiB and explicitly defers the cap as `D-023-01`.
Failure scenario: Larger tables, wider vectors, or WASM targets can scale memory with `N` and vector width and hit excessive RSS or OOM before create finishes.
Correction: Keep `D-023-01` open and add a cap or SwapVec-aware bound before treating large/WASM coverage as complete.
Verification: Re-run the 14k fixture and at least one larger or constrained-memory case after the cap lands; confirm bounded RSS and preserved `store_get_count` improvement.
Deferrable: Yes

## Completeness Sweep
Searched runtime/query scope for `TODO`, `FIXME`, `XXX`, `HACK`, `placeholder`, `stub`, `unimplemented!`, and `panic!("TODO")`. I found no track-scope placeholders in the touched HNSW code. The only matches were unrelated test noise in `cozo-core/src/runtime/tests.rs`.

## Wiring and Regression Review
`::hnsw create` still flows through `SysOp::CreateVectorIndex` with a base-relation write lock in [db.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/db.rs:1288>), then `create_hnsw_index` scans tuples once and reuses one cache across the create loop in [relation.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1209>). Neighbor misses in `ensure_key` are now measured via `time_store_get` in [hnsw.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:285>).

Incremental updates remain separate: [stored.rs](</C:/dev/CozoDB-redux/cozo-core/src/query/stored.rs:443>) still calls `hnsw_put`, which allocates a fresh per-put cache in [hnsw.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1064>). I saw no search-path global cache, no threshold retune, and no on-disk format or `BREAKING.md` drift.

## Verification Evidence
Observed now:
- `git diff origin/main` shows the expected runtime/doc changes plus untracked [hnsw_prefetch_test.rs](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_prefetch_test.rs:1>).
- `git diff origin/main -- cozo-core/src/query/stored.rs` is empty.
- Raw [prefetch_ef100.json](</C:/dev/CozoDB-redux/conductor/track023-hnsw-prefetch/raw/prefetch_ef100.json:1>) reports `store_get_count: 0`, `cache_instances: 1`, `cache_peak: 14000`.

Reported by orchestrator:
- `cargo clippy -p cozo --all-targets --all-features -- -D warnings` passed.
- `cargo nextest run --lib --bins --workspace` passed: 187 passed, 3 skipped.
- `cargo check -p cozo --no-default-features --features compact-single-threaded` passed.
- `cargo fmt --all -- --check` was not rerun after the last markdown/doc edits.

Not verifiable here:
- I did not rerun cargo gates in this read-only environment.
- Ledgerful and AI-Brains signals were unavailable.

## Deferred Candidates
No new deferred candidate. Existing [D-023-01](</C:/dev/CozoDB-redux/docs/deferred.md:31>) is the correct non-blocking residual.

## Completion Decision
`PASS WITH DEFERRED P3`.

The core track objective is satisfied: the create path now reuses one create-scoped cache, incremental `stored.rs` behavior is preserved, the win is measured with `store_tx.get`, and the 14k fixture evidence supports the claimed `19.9 min -> 12.6 min` improvement. The only remaining issue is the already-recorded low-severity RAM-cap follow-up `D-023-01`.
