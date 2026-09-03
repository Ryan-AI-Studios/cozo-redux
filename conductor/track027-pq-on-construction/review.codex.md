# Track Completion Audit — track027-pq-on-construction
## Verdict: PASS

## Scope Reviewed
Working tree on `track/027-pq-on-construction` against `origin/main`. `origin/main...HEAD` is empty, so this review is of the current uncommitted working tree diff, not a branch-only commit range. I reviewed [spec.md](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/spec.md), [plan.md](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/plan.md), the 027 code/tests, and the track/docs updates.

## Requirement and DoD Matrix
| Requirement / DoD | Status | Evidence | Tests | Gap |
|---|---|---|---|---|
| Req 1: absorb D-012-01 without closing it on construction-PQ kill | Met | [design.md:5](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/design.md:5>), [design.md:9](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/design.md:9>), [docs/deferred.md:18](</C:/dev/CozoDB-redux/docs/deferred.md:18>), [conductor.md:31](</C:/dev/CozoDB-redux/conductor/conductor.md:31>) | Observed by code/doc inspection | None |
| Req 2: explicit opt-in; default F32 exact path unchanged | Met | [results.md:25](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:25>), [relation.rs:1152](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1152>), [relation.rs:1167](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1167>) | Observed by code/doc inspection | None |
| Req 3: `compact-single-threaded` still builds; RAM documented | Met | RAM note in [design.md:27](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/design.md:27>) and [results.md:40](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:40>); compact claim in [results.md:42](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:42>) | Reported by orchestrator: `cargo check -p cozo --no-default-features --features compact-single-threaded` passed | I did not rerun the build in this read-only session |
| Req 4: HITL before any on-disk layout change | Met | [design.md:25](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/design.md:25>), [results.md:25](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:25>) | Observed by code/doc inspection | No layout change landed |
| Req 5: if keep construction PQ, no GPU requirement | Met | Construction-PQ was killed; [design.md:25](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/design.md:25>) | N/A | N/A |
| Req 6: `train_pq` must reject non-L2 | Met | [hnsw.rs:1736](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1736>) | Reported: `hnsw_train_pq_rejects_cosine`; test code at [hnsw_pq_construction_test.rs:176](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_pq_construction_test.rs:176>) | None |
| Req 7: `num_centroids` must be `1..=256` in parser and trainer | Met | [sys.rs:668](</C:/dev/CozoDB-redux/cozo-core/src/parse/sys.rs:668>), [hnsw.rs:1748](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1748>) | Reported: `hnsw_train_pq_rejects_centroid_overflow`; test code at [hnsw_pq_construction_test.rs:198](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_pq_construction_test.rs:198>) | None |
| DoD: measured kill gates (a) and (b) at 14k×768 | Met | [results.md:5](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:5>), [results.md:14](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:14>), [raw/pq_14k.json](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/raw/pq_14k.json) | Observed raw artifact and doc alignment | None |
| DoD: kill/keep decision + no silent format break | Met | [results.md:25](</C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md:25>), [relation.rs:1167](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1167>) | Observed by code/doc inspection | None |
| DoD: re-rank landed | Met | [hnsw.rs:1629](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1629>) | Reported: `hnsw_pq_search_reranks_with_exact_l2`; test code at [hnsw_pq_construction_test.rs:217](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw_pq_construction_test.rs:217>) | None |

## Findings
No P0-P3 findings.

## Completeness Sweep
No 027-scope `TODO`/`FIXME`/`HACK`/`unimplemented!` placeholders were present in the shipped implementation paths or final 027 track docs I reviewed.

## Wiring and Regression Review
`::hnsw train_pq` now flows from parser validation in [sys.rs:663](</C:/dev/CozoDB-redux/cozo-core/src/parse/sys.rs:663>) into runtime validation in [hnsw.rs:1732](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1732>). PQ query-time search still builds the L2 LUT, then re-scores PQ survivors with exact `v_dist` before `bind_distance` is emitted in [hnsw.rs:1629](</C:/dev/CozoDB-redux/cozo-core/src/runtime/hnsw.rs:1629>). Default create remains exact and still writes `pq: None` in [relation.rs:1167](</C:/dev/CozoDB-redux/cozo-core/src/runtime/relation.rs:1167>). I found no silent storage-format or API-surface drift relative to the track claims.

## Verification Evidence
Observed now:
- [raw/pq_14k.json](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/raw/pq_14k.json) matches the documented gate numbers in [results.md](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md).
- The RAM note required by Req 3 is present in both [design.md](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/design.md) and [results.md](/C:/dev/CozoDB-redux/conductor/track027-pq-on-construction/results.md).
- `D-012-01` remains open and explicitly not closed in [docs/deferred.md:18](</C:/dev/CozoDB-redux/docs/deferred.md:18>).

Reported by orchestrator:
- `hnsw_pq_*` unit tests passed.
- Ignored 14k release test passed.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `cargo check -p cozo --no-default-features --features compact-single-threaded` passed.

Not verifiable here:
- I did not rerun Cargo gates in this read-only session.
- `ledgerful doctor --json` and `ledgerful ledger status --compact` both failed locally with `unable to open database file`.
- `ai-brains preflight --summary` failed locally because `AI_BRAINS_KEY` is missing.

## Deferred Candidates
None proposed. The existing residual [D-012-01](</C:/dev/CozoDB-redux/docs/deferred.md:18>) is correctly preserved and is not a new review finding.

## Completion Decision
Track 027 satisfies its requirements and DoD in the current working tree. The prior Req 3 documentation gap is fixed, the compact-single-threaded claim is at least backed by reported gate evidence, construction-PQ is killed without collapsing `D-012-01`, and the shipped code/doc/test wiring matches the track’s stated outcome.
