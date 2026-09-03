# Track Completion Audit — track025-hnsw-incremental-optimize
## Verdict: FAIL

## Findings

[P2] B2 wall-clock evidence is measured on 4,000 singleton writes, not the claimed 8×500 live `:put` batches
Confidence: High
Requirement: Spec requirement 1 and plan phase 1 require Build B2 to be mixed live appends in `Δ` batches and to supply the quality-vs-time comparison.
Location: [hnsw_incremental_optimize_test.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_incremental_optimize_test.rs:99), [hnsw_incremental_optimize_test.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_incremental_optimize_test.rs:112), [hnsw_incremental_optimize_test.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_incremental_optimize_test.rs:155), [results.md](C:/dev/CozoDB-redux/conductor/track025-hnsw-incremental-optimize/results.md:26), [results.md](C:/dev/CozoDB-redux/conductor/track025-hnsw-incremental-optimize/results.md:36), [db.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/db.rs:410), [lib.rs](C:/dev/CozoDB-redux/cozo-core/src/lib.rs:602)
Problem: `put_range` loops row-by-row and calls `DbInstance::run_script(..., ScriptMutability::Mutable)` once per row. The report still describes Build B2 as “8×500 live `:put`”.
Evidence: The harness increments `batches`, but each “batch” is implemented as 500 separate `put_embedding` calls. `DbInstance::run_script` executes one script per call; separate transactional batching is exposed through `MultiTransaction`. The report itself says B2 was slower because of “4,000 separate write txns.”
Failure scenario: The reported `14.8 min` B2 wall time is not a valid measurement of the spec’s batch-append cadence, so the time side of requirement 1 remains unverified. The quality result (`recall@10 vs A = 0.56`) still supports killing the approach, but the “and slower than rebuild” conclusion is overstated.
Correction: Re-measure B2 with actual 500-row live append batches, or restate the track as a quality-only kill and remove the timing claim.
Verification: Re-run the 14k fixture with one mutable append transaction per 500-row batch and regenerate `results.md` / `raw/incremental_14k.json`.
Deferrable: No

## Scope Reviewed

`origin/main` vs the current working tree on `track/025-hnsw-incremental-optimize`, plus the track artifacts:
[spec.md](C:/dev/CozoDB-redux/conductor/track025-hnsw-incremental-optimize/spec.md:1), [plan.md](C:/dev/CozoDB-redux/conductor/track025-hnsw-incremental-optimize/plan.md:1), [results.md](C:/dev/CozoDB-redux/conductor/track025-hnsw-incremental-optimize/results.md:1), [raw/incremental_14k.json](C:/dev/CozoDB-redux/conductor/track025-hnsw-incremental-optimize/raw/incremental_14k.json:1), [hnsw_incremental_optimize_test.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_incremental_optimize_test.rs:1), [mod.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/mod.rs:1), [stored.rs](C:/dev/CozoDB-redux/cozo-core/src/query/stored.rs:434), [relation.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1020), [sys.rs](C:/dev/CozoDB-redux/cozo-core/src/parse/sys.rs:29).

## Requirement and DoD Matrix

| Requirement | Status | Evidence | Tests | Gap |
|---|---|---|---|---|
| Req 1: Quality vs time table with A vs live-appended B2 | Partial | `results.md` has A/B2 table and recall vs A; harness uses a built index then live appends | Static review of harness and raw JSON | B2 timing is from 4,000 singleton writes, not the documented `8×500` batch cadence |
| Req 2: Named CozoScript API if keep | Met | Track outcome is kill; no `::hnsw optimize`; `SysOp` has no such variant | Static review | None |
| Req 3: Two-sided handshake noted; Ledgerful patch out of scope | Met | Handshake documented in spec/results | Static review | None |
| Req 4: Keep public create options working | Not verifiable | No production create-path change observed; smoke drop/create test added | Test file present, but not run by me | No local execution evidence |
| Req 5: If kill, document periodic recreate | Met | Kill policy documented in spec/results | Static review | None |
| DoD: Quality vs time table in track dir | Partial | `results.md`, raw JSON present | Static review | Same B2 timing defect |
| DoD: Kill/keep and API decision | Met | Kill recorded; no new sysop required | Static review | None |
| DoD: If keep, tests vs rebuild recall + HNSW suite | N/A | Track is kill-only | N/A | None |

## Completeness Sweep

No placeholder markers were found in the reviewed track-scoped files (`TODO`, `FIXME`, `HACK`, `stub`, `unimplemented!`, `panic!("TODO")`).

## Wiring and Regression Review

The reviewed wiring is coherent for a measurement-only spike:

- Live appends flow through `:put snippet_embedding` into `update_in_hnsw` in [stored.rs](C:/dev/CozoDB-redux/cozo-core/src/query/stored.rs:434), which calls `hnsw_put`.
- Full rebuild still goes through `create_hnsw_index` in [relation.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1020), which populates the index by per-tuple HNSW insertion.
- No new sysop was added; [sys.rs](C:/dev/CozoDB-redux/cozo-core/src/parse/sys.rs:29) still has no `optimize` op.
- No storage-format change was observed.

## Verification Evidence

Observed now:
- `raw/incremental_14k.json` reports `build_a_create_ms = 691960.3442`, `build_b2_total_ms = 890094.0327`, `recall_at_10_vs_a = 0.5600000000000002`.
- The new test module is wired into [mod.rs](C:/dev/CozoDB-redux/cozo-core/src/runtime/mod.rs:22).
- `ledgerful doctor --json` and `ledgerful ledger status --compact` both failed locally with `unable to open database file`.
- `ai-brains preflight --summary` failed locally with `VAULT_KEY_MISSING`.

Reported by orchestrator:
- `cargo clippy -p cozo --all-targets --all-features -- -D warnings` passed.
- The ignored 14k release test passed.
- `cargo nextest` is still being run.

Not verifiable:
- I did not observe any cargo/test execution in this review session.

## Deferred Candidates

None. The open issue is a P2 evidence defect and should be fixed before completion.

## Completion Decision

The track’s quality evidence is strong enough to support a kill on recall grounds, and the “no new sysop on kill” requirement is satisfied. The review still fails because the shipped B2 timing evidence does not match the required workload: the code measures 4,000 singleton mutable writes while the report claims `8×500` live batch appends. Until that is corrected or the timing claim is narrowed, requirement 1 and the table DoD are only partially satisfied.
