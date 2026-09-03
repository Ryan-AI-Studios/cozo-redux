# Track 027: Product quantization (PQ) on construction

## Status

**KILL construction-PQ.** 14k dist is 14.55% of create; `train_pq` is 20 s post-hoc. L2 guard, centroid `1..=256`, and exact re-rank landed. Convert won’t-do. D-012-01 stays open.

## Objective

Decide whether PQ can participate **during** `::hnsw create` (construction distances / encoding) without a silent format break, or whether post-create `::hnsw train_pq` remains the only path. Absorb Track 012 gaps: re-rank, cosine PQ, `hnsw_convert_to_pq`.

Product north star: Ledgerful create still walks **full F32** and never calls `train_pq`. Construction PQ is only worth it if it beats exact L2 create on time or RAM at 14k×768 with an explicit opt-in.

## Plan-time snapshot (2026-09-02) — re-verify at execute

Live (Track 012 shipped):

| Piece | Reality |
| :--- | :--- |
| Create | `HnswIndexManifest.pq: None` always (`relation.rs` ~1146–1161). Construction uses exact `v_dist`. |
| Train | `::hnsw train_pq` (not `train-residual`). Post-create. K-means L2, 25 iters, F32. Does **not** check `manifest.distance`. `num_centroids` has no upper bound in parser or `train_pq`; encode pushes `best_c as u8` (`hnsw.rs` ~175). |
| Search approx | Squared-L2 LUT only. `PQ search only supported for F32 vectors`. |
| Re-rank | **Missing.** Final scores can be approx. After `train_pq`, `ensure_key` **still** loads full vectors in `hnsw_search_level` (`hnsw.rs` ~822–825). OpenCode m1’s “no full-vector fetch” claim is **wrong**. Re-rank is exact L2 on final `k` survivors (cheap) — still a D-012-01 item. |
| Cosine PQ | **Missing.** Exact cosine exists; PQ path is L2-shaped. Silent L2 LUT on a cosine index is possible today. |
| `hnsw_convert_to_pq` | **No symbol.** Ledgerful never needs convert; the design note may kill convert as “won’t do” **without** closing D-012-01. |
| On-disk | Codebook: MessagePack `PqCodebook` at layer sentinel `i64::MAX`. Codes at `i64::MAX - 1`. |
| Construction `k_dist` | `hnsw_select_neighbours_heuristic` uses `vec_cache.k_dist` on full cached vectors (`hnsw.rs` ~736). ADC cannot replace `k_dist`. Design constraint for construction-PQ only — not a substitute for measuring the kill gates. |

Placeholder wording `::hnsw train-residual` is **wrong**. Use `::hnsw train_pq`.

Ledgerful: zero `train_pq`. Index L2 + query `cos_dist` rerank — construction PQ would need a metric story, not just speed.

## Spike / kill

**Split kill gates** (do not collapse them):

| Gate | Measures | Effect of kill |
| :--- | :--- | :--- |
| **(a)** post-hoc `train_pq` vs L2 create | Time `::hnsw create` then `::hnsw train_pq` at 14k×768 | Killing (a) does **not** automatically kill construction-PQ. |
| **(b)** construction-PQ estimate | LUT / encode vs 021 **dist** share (not total create). Honor the `k_dist`/ADC constraint when estimating. | Killing construction-PQ does **not** close D-012-01 and must **not** park re-rank / cosine `train_pq` guard / convert as “won’t do”. |

Do **not** treat Agy “certain kill / 32× MAC” as a substitute for measurement (O01). Measure (a) and (b).

**Keep construction-PQ if:** create or query memory/time improves with **explicit opt-in** and recall still meets the 025/026-style band, **and** gate (b) beats exact L2 create at 14k×768.

**Kill construction-PQ if:** gate (b) loses; or format/WASM/SQLite cost exceeds benefit; or 021–023 already win; or cosine/L2 mismatch makes PQ useless for Ledgerful.

**Keep (independent of construction-PQ):** `train_pq` L2 guard; `num_centroids` `1..=256`; re-rank (until separately killed or a follow-up is minted). Convert may be “won’t do” in the design note without closing the row.

## Requirements

1. Absorb **D-012-01**: construction-time PQ vs post-create `train_pq`; re-rank; cosine; convert. Construction-PQ kill ≠ close the row.
2. Explicit opt-in; default F32 exact path unchanged.
3. `compact-single-threaded` still builds; document RAM for codebook training.
4. HITL before any on-disk layout change (`BREAKING.md`).
5. If keep construction PQ: construction still must not require GPU.
6. Even if construction-PQ dies: `ensure!(manifest.distance == L2)` (or equivalent) on `train_pq` (silent L2 LUT on cosine indexes).
7. `num_centroids` must be `1..=256` in the parser and in `train_pq` (encode pushes `best_c as u8`).

## Out of scope

GPU (**028**). Replacing HNSW. Silent schema change. Editing Ledgerful.

## Dependencies

**021**. Track **012** (shipped). Optional **022** (exact L2 faster → PQ less attractive).

## §9 Deferred

| ID | Action | Notes |
| :--- | :--- | :--- |
| **D-012-01** | **Absorb** | This track owns construction PQ, re-rank, cosine PQ / `train_pq` L2 guard, `hnsw_convert_to_pq`. Killing construction-PQ does **not** close the row. Convert may be “won’t do” in the design note without closing the row. |
| D-009-01 | Decline | Owner **029**. |

### Fold-in (2026-09-02)

| Id | Source | Sev | Disposition | Action |
| :--- | :--- | :--- | :--- | :--- |
| opencode-M1 | OpenCode | M | Agree — fold | Split kill gate: (a) post-hoc `train_pq` vs L2 create; (b) construction-PQ estimate (LUT vs 021 dist share). Killing (a) ≠ kill construction-PQ; killing construction-PQ ≠ close D-012-01. |
| opencode-m1 | OpenCode | m | Decline | After `train_pq`, `ensure_key` still loads full vectors in `hnsw_search_level` (~822–825). Keep that spec fact. Re-rank remains exact L2 on final `k` survivors. |
| agy-T027-B01 | Agy | B | Agree — fold | Construction-PQ kill must not park re-rank / cosine-guard / convert as “won’t do” or close D-012-01. Those stay in this track until separately killed or a follow-up is minted. |
| agy-T027-M01 | Agy | M | Agree — fold | Even if construction-PQ dies, `ensure!(manifest.distance == L2)` (or equivalent) on `train_pq` is in-scope D-012-01. |
| agy-T027-M02 | Agy | M | Agree — fold | `num_centroids` must be `1..=256`. Parser + `train_pq`. |
| agy-T027-O01 | Agy | O | Decline (kill substitute) / Agree (constraint) | Do not skip measurement. Fold `k_dist` cannot use ADC as a construction-PQ design constraint only. |
| Close D-012-01 on construction-PQ kill | Agy / plan Phase 1 | — | Decline | See B01. Update `docs/deferred.md` notes only. |

## Last-PR Cursor comments

**N/A this track.** Empty GitHub PR scan (see 021). Track 012 plan Phase 3 re-rank remains unchecked — folded here, not reopened as 012.

## Tools (planning)

ledgerful + ai-brains used (vault empty). Live `hnsw_train_pq` / encode / LUT confirmed.

## Testing / Definition of Done

- [x] Design: during-create vs `train_pq`-only (this track dir). Cover re-rank (exact L2 on final `k`), cosine `train_pq` guard, convert (Ledgerful: none; convert may be “won’t do” without closing D-012-01).
- [x] Measured kill gates (a) and (b) at 14k×768. Do not use MAC-count “certainty” as the gate.
- [x] Kill or keep construction-PQ + HITL if format change. Construction-PQ kill does **not** close D-012-01.
- [x] In-scope even if construction-PQ is killed: `train_pq` L2 `ensure!`; `num_centroids` `1..=256` in parser + `train_pq`.
- [x] If keep construction-PQ: tests for opt-in path + default F32 unchanged. (N/A — killed. Default F32 path unchanged.)

## Hard locks

No silent format break. Default create API unchanged.
