# Track 023 plan

## Status

**In progress.** 021 kept this track. Create reuses one `VectorCache`; hashmap (not columnar).

## Phases (map to DoD)

### 1. Confirm double-fetch (DoD: 021 numbers)

- Use 021 **`store_tx.get`** count vs N tuples vs cache maps created.
- If 021 killed 023, record kill and stop.

### 2. Create-scoped cache prototype (DoD: measurable cut)

- Add `hnsw_put_with_cache` (or equivalent) used **only** by `create_hnsw_index`. Leave `query/stored.rs` incremental `hnsw_put` on a fresh per-put cache.
- Retain one `VectorCache` across the create loop. Self-warming: `vec_cache.insert` on insert (`hnsw.rs` ~351).
- Do **not** `ensure_key`/`handle.get` every scanned tuple again. Optional: copy vectors already in `TempCollector` into the cache without a store get.
- Measure peak RSS vs 021. Document F64/dim scaling. If keep-as-default: design a cap (follow-up, not spike DoD).

### 3. Decide columnar vs hashmap (DoD: keep/keep-not)

- HashMap `CompoundKey → Vector` is the live type; columnar buffer only if execute shows hashmap overhead ≫ decode (include CompoundKey↔usize translation cost).
- Do not persist the buffer.

## Files (expected)

- `cozo-core/src/runtime/hnsw.rs` (`hnsw_put`, `VectorCache`)
- `cozo-core/src/runtime/relation.rs` (`create_hnsw_index`)
- Read: `cozo-core/src/query/stored.rs` (do not change incremental path unless HITL)

## Gate

fmt / clippy / lib+bins. No format break. `compact-single-threaded` builds.

## Execute notes

Search-path `hnsw_knn` may keep per-query caches. Do not silently make search retain a process-global vector cache.
