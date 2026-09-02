# Deferred items (CozoDB-redux)

Track-scoped findings and intentional deferrals that are **not** blocking completion, but must
not be lost. Update when fixed or when a track owns the work.

Med/high never belong here — they block the owning track.

## From Track 009 (search performance)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-009-01 | — | Outer-loop parallel KNN (`StoreTx: is_concurrent_read_safe()`) | Phase 3 deferred on purpose. **029** spec Ready — not started. Live API is `SessionTx::hnsw_knn`. `SessionTx` is Sync (`StoreTx: Sync`); `StoreTx` is not Send. SQLite concurrent-read likely false (statement mutex convoy). May kill after Phase 1 workload note (Ledgerful is single-query); do **not** close this row in the 2026-09-02 fold-in. | **029** |

## From Track 012 (PQ)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-012-01 | — | PQ on construction; re-rank; cosine; `hnsw_convert_to_pq` | Ledgerful create does not `train_pq`. Live sysop is `::hnsw train_pq` (no train-residual / convert). **027** spec Ready — not started. Split kill gates: (a) post-hoc `train_pq` vs L2 create; (b) construction-PQ estimate. Killing construction-PQ does **not** close this row and does **not** park re-rank / cosine `train_pq` guard / convert as “won’t do”. Convert may be “won’t do” in the design note without closing the row. | **027** |

## From Track 021 (HNSW create baseline)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-021-01 | low | Graph/heaps remainder is 54.6% of create at ef100 14k×768 SQLite Windows | Neighbor walk, heaps, key encode, `get`, shrink. Not dist / ensure_key / put / commit. No 022–028 spike owns it. Plan after 023 if still the pain. | future (not 022–028) |
| D-021-02 | low | `COZO_HNSW_CREATE_STATS` is process-global | Tests use a Drop guard. Default off in production. Isolation is test-only. | 021 residual |

## Hygiene (not a track)

None open. D-HYG-01 (uncommitted post-Codex remediations) closed when those changes landed on `main`.
