# Track Completion Audit — track027-pq-on-construction
## Verdict: FAIL

## Scope Reviewed
Working tree on `track/027-pq-on-construction` against `origin/main`. I read `spec.md`, `plan.md`, `design.md`, `results.md`, `raw/pq_14k.json`, and reviewed the implementation/doc changes in `cozo-core/src/parse/sys.rs`, `cozo-core/src/runtime/hnsw.rs`, `cozo-core/src/runtime/relation.rs`, `cozo-core/src/runtime/mod.rs`, `cozo-core/src/runtime/hnsw_pq_construction_test.rs`, `conductor/conductor.md`, and `docs/deferred.md`.

`ledgerful doctor --json` and `ledgerful ledger status --compact` both failed here with `unable to open database file`. `ai-brains preflight --summary` failed with `VAULT_KEY_MISSING`. I did not modify files or Git state.

## Requirement and DoD Matrix
| Item | Status | Evidence | Tests | Gap |
|---|---|---|---|---|
| Req 1: absorb D-012-01 without closing it on construction-PQ kill | Met | `design.md:5-25`, `results.md:12-34`, `docs/deferred.md:18` | Reported by orchestrator | Cosine ADC remains intentionally open, and the row stays open |
| Req 2: explicit opt-in; default exact-F32 create unchanged | Met | `relation.rs:1152-1167` keeps `pq: None`; no create-time PQ path added | Code inspection | None |
| Req 3: `compact-single-threaded` still builds; document RAM for codebook training | Partial | `spec.md:53`; `hnsw.rs:1756-1830` still materializes samples and tuples | None observed now | No RAM note found in final 027 docs; `compact-single-threaded` not rerun here |
| Req 4: HITL before any on-disk layout change | Met | `design.md:25`; no manifest/storage layout diff beyond existing PQ path | Code inspection | None |
| Req 5: if kept, construction PQ must not require GPU | N/A | Construction-PQ was killed | N/A | N/A |
| Req 6: `train_pq` must reject non-L2 | Met | `hnsw.rs:1736-1740` | Reported: `hnsw_train_pq_rejects_cosine` | None |
| Req 7: `num_centroids` must be `1..=256` in parser and trainer | Met | `parse/sys.rs:663-673`, `hnsw.rs:1748-1752` | Reported: `hnsw_train_pq_rejects_centroid_overflow` | None |
| DoD: design covers create-vs-train, re-rank, cosine guard, convert | Met | `design.md:5-25` | N/A | None |
| DoD: measured split kill gates (a) and (b) at 14k×768 | Met | `results.md:5-25`, `raw/pq_14k.json` | Reported: ignored 14k release test passed | None |
| DoD: kill/keep decision without silent format break | Met | `results.md:23-25`, `conductor.md:9,19,31` | Code inspection | None |
| DoD: even on kill, L2 guard and centroid bound land | Met | `hnsw.rs:1736-1752`, `parse/sys.rs:663-673` | Reported unit tests | None |
| DoD: re-rank lands after PQ search | Met | `hnsw.rs:1629-1637`; `results.md:31-33` | Reported: `hnsw_pq_search_reranks_with_exact_l2` | None |

## Findings
[P3] Requirement 3 is still undocumented
Confidence: High
Requirement: `spec.md` Requirement 3
Location: `conductor/track027-pq-on-construction/spec.md:53`; `conductor/track027-pq-on-construction/design.md:5`; `conductor/track027-pq-on-construction/results.md:1`; `cozo-core/src/runtime/hnsw.rs:1756`; `cozo-core/src/runtime/hnsw.rs:1830`
Problem: The track is marked complete, but the final 027 docs do not document PQ-training RAM behavior even though `hnsw_train_pq` still materializes all sampled vectors and then all tuples in memory.
Evidence: Requirement 3 explicitly asks to “document RAM for codebook training.” `design.md` and `results.md` record timing, kill gates, and landed guards/re-rank, but no memory note. The implementation still builds `all_samples: Vec<Vec<f32>>` and later `all_tuples: Vec<Tuple>`.
Failure scenario: A user runs post-create `::hnsw train_pq` on a larger relation without any warning about memory scaling and hits avoidable RAM pressure/OOM; the written requirement remains only partially satisfied.
Correction: Add a short RAM note to the final 027 docs describing current `train_pq` memory behavior, and only claim `compact-single-threaded` compatibility if it was actually verified.
Verification: Re-read the 027 docs for the RAM note; if the build claim is kept, run the intended `compact-single-threaded` build before re-review.
Deferrable: No

## Completeness Sweep
No 027-scope `TODO`/`FIXME`/`HACK`/`unimplemented!` placeholders were found in the changed implementation paths. I found no silent format/storage change: default create still writes `pq: None`, and no new create-time PQ/sysop path was added.

## Wiring and Regression Review
Parser enforcement is present in `parse/sys.rs:663-673`. Runtime enforcement is present in `hnsw.rs:1736-1752`. PQ search now re-scores survivors with exact `v_dist` before `bind_distance` is emitted in `hnsw.rs:1629-1637`. `relation.rs:1152-1167` preserves the default exact create path. `docs/deferred.md:18` and `conductor.md:31` correctly keep D-012-01 open instead of collapsing the construction-PQ kill into a full PQ closure.

## Verification Evidence
Observed now: `raw/pq_14k.json` matches the documented numbers for gate (a) and gate (b); code and docs align on killing construction-PQ while keeping L2 guard, centroid bound, and exact re-rank.

Reported by orchestrator: `hnsw_pq_*` unit tests passed; the ignored 14k release test passed; `fmt`, `clippy`, and `nextest` are being run separately.

Not verifiable here: I did not rerun Cargo gates or a `compact-single-threaded` build in this read-only session.

## Deferred Candidates
None. The open item above is an explicit, easy-to-fix requirement gap and should not be deferred.

## Completion Decision
Track 027 is close, but I do not consider it complete yet. The core kill decision and code landing are aligned with the spec, but Requirement 3 is still only partially satisfied because the final track docs do not document PQ-training RAM behavior. Once that is fixed and the `compact-single-threaded` claim is either evidenced or removed, this is ready for re-review.
