# Deferred items (CozoDB-redux)

Track-scoped findings and intentional deferrals that are **not** blocking completion, but must
not be lost. Update when fixed or when a track owns the work.

Med/high never belong here — they block the owning track.

## From Track 009 (search performance)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-009-01 | — | Outer-loop parallel KNN (`StoreTx: is_concurrent_read_safe()`) | **029** KEEP: trait default false; SQLite explicit false; mem Reader / RocksDB / fjall true; chunked lazy rayon (threshold 8, `map_init` stacks). Ledgerful still single `$query_vec`; generic Cozo parent-join is the workload. Close when **029** squash-merges. | **029** |

## From Track 012 (PQ)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-012-01 | — | PQ on construction; re-rank; cosine; `hnsw_convert_to_pq` | **027** killed construction-PQ (14k dist 14.55% of create). Landed: `train_pq` L2 guard, `num_centroids` 1..=256, exact `v_dist` re-rank. Convert documented **won’t do**. Cosine ADC still absent (L2 LUT still L2-only). **Do not close** this row. | **027** residual |

## From Track 021 (HNSW create baseline)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-021-01 | low | Graph/heaps remainder is 54.6% of create at ef100 14k×768 SQLite Windows | After **023**, ensure_key is 0.58%; graph/heaps is **82.5%** of the shorter create (~623 s, similar absolute). Neighbor walk, heaps, key encode, shrink. No 022–028 spike owns it. | future (not 022–028) |
| D-021-02 | low | `COZO_HNSW_CREATE_STATS` is process-global | Tests use a Drop guard plus `with_exclusive`. Default off in production. Isolation is test-only. | 021 residual |

## From Track 023 (HNSW prefetch)

| ID | Severity | Item | Notes | Owner |
|---|---|---|---|---|
| D-023-01 | low | Create-wide VectorCache has no RAM cap | Default create path retains all vectors (14k×768 peak WS ~119 MiB). Spec allowed documenting RSS on the spike; hard cap / SwapVec-aware bound for large/WASM tables is follow-up. | future (WASM / large N) |

## Hygiene (not a track)

None open. D-HYG-01 (uncommitted post-Codex remediations) closed when those changes landed on `main`.
