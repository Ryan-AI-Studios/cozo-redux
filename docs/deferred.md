# Deferred items (CozoDB-redux)

Track-scoped findings and intentional deferrals that are **not** blocking completion, but must
not be lost. Update when fixed or when a track owns the work.

Med/high never belong here — they block the owning track.

## From Track 009 (search performance)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-009-01 | — | Outer-loop parallel KNN (`StoreTx: is_concurrent_read_safe()`) | Phase 3 deferred on purpose | **029** |

## From Track 012 (PQ)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-012-01 | — | PQ on construction; re-rank; cosine; `hnsw_convert_to_pq` | Ledgerful create does not `train_pq` | **027** (spike; kill if training ≥ L2 create) |

## Hygiene (not a track)

None open. D-HYG-01 (uncommitted post-Codex remediations) closed when those changes landed on `main`.
