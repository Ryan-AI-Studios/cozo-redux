# Track 025 plan

## Status

**KILL.** B2 = N0 create then mixed live `$data` `:put` batches. Smoke (N=48 dim=8) recall@10 vs A = 1.0. 14k: recall 0.54, B2 16.0 min > A 11.9 min.

## Phases (map to DoD)

### 1. Quality vs time (DoD: table)

- Same 14k×768 seeded fixture as 021 (Nomic embeddings not required).
- **Build A:** `::hnsw drop` + `::hnsw create` over all 14k rows (today’s Ledgerful rebuild).
- **Build B2:** start from a built index, then mixed appends on a **live** index (N0=10k then 8×500). Do **not** use same-order empty insert-all as the quality test (`create_hnsw_index` is already that loop).
- Assert incremental puts actually insert (canary-hash early-return can no-op duplicates).
- One mutable `:put` with `$data` per batch (Ledgerful cadence), not per-row `run_script`.
- Measure wall clock. Primary: recall@10 vs A’s neighbors at `ef: 100`. Optional brute-force column; do not replace vs-A.

### 2. API decision (DoD: keep/kill)

- If B2 ≈ A: document “Ledgerful can raise threshold / skip drop” — no Cozo sysop required; still record the handshake. Do not patch Ledgerful `HnswRefreshPlan` unless owner asks.
- If gap: design `::hnsw optimize` (parser `sys.rs`) vs kill and keep drop+create.
- If keep optimize: design note covers reverse-link / in-degree (shrink is outgoing-only), exclusive write-txn duration, tombstone compaction + degree vs neighbour-count under multi-batch put.
- If kill: document periodic recreate.

### 3. Implement only on keep (DoD: tests)

- Parser + sysop tests; recall fixture; existing HNSW tests.

## Files (expected)

- `cozo-core/src/runtime/hnsw.rs`, `query/stored.rs`
- `cozo-core/src/parse/sys.rs` if new sysop
- This track dir: `results.md`

## Gate

Existing HNSW tests. New recall fixture if keep. fmt / clippy / nextest if code lands.

## Execute notes

Track 010 delete-repair is not a substitute for optimize-on-append. Do not reopen 010. Do not gate 025 on 023. Do not invent 180s vs 35s / 2M-lookup figures.
