# Track 027 plan

## Status

**KILL construction-PQ.** Gate (a): create 11.6 min, `train_pq` 20 s. Gate (b): dist 14.55% of create; encode+LUT 1.3 s. L2 guard / centroids / re-rank landed.

## Phases (map to DoD)

### 1. Cost the existing train (DoD: split kill gates)

- Gate **(a)** post-hoc: on the 14k×768 fixture, time `::hnsw create` then `::hnsw train_pq`.
- Gate **(b)** construction-PQ estimate: LUT-build / encode vs 021 **dist** share (not total create). Construction heuristic selection uses `k_dist` on full cached vectors; ADC cannot replace `k_dist` — treat that as a design constraint when estimating, not as a skip-measurement argument.
- Killing (a) does **not** automatically kill construction-PQ.
- If (b) loses, **kill construction-PQ** and document `train_pq` as post-hoc only. Do **not** park re-rank / cosine-guard / convert as “won’t do” and do **not** close D-012-01.

### 2. Design (DoD: absorb D-012-01; search-side stays)

- Construction distances: stay exact vs use PQ codes mid-build (quality risk). `k_dist` stays full-vector.
- Re-rank: exact L2 on final `k` survivors (Track 012 Phase 3; cheap because `ensure_key` already loaded vectors). Remains in this track until separately killed or a follow-up is minted.
- Cosine: do not pretend L2 LUT is cosine. Even if construction-PQ dies, add `ensure!(manifest.distance == L2)` (or equivalent) to `train_pq`.
- `num_centroids` `1..=256` in `parse/sys.rs` and `hnsw_train_pq` (encode pushes `best_c as u8`).
- Convert: only if a real migration need exists. Ledgerful: none. The design note may kill convert as “won’t do” without closing D-012-01.

### 3. Implement (DoD: tests)

Always in-scope (even if construction-PQ is killed):

- `train_pq` L2 distance guard.
- `num_centroids` `1..=256` (parser + `train_pq`).
- Re-rank unless a later kill / minted follow-up says otherwise.

Construction-PQ only on keep:

- Opt-in sysop or create config; default `pq: None`.
- Tests: default F32 path identical; opt-in recall fixture.

## Files (expected)

- `cozo-core/src/runtime/hnsw.rs`, `parse/sys.rs`, maybe storage blobs
- This track dir: `design.md` / `results.md`

## Gate

No silent format break. fmt / clippy / tests if code lands.

## Execute notes

Do not name the sysop `train-residual`. Live name is `train_pq`.
