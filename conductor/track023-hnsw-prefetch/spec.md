# Track 023: Prefetch / cache-fill before Track 009 rayon

## Status

**In progress.** **KEEP** — create-scoped `VectorCache`; incremental `hnsw_put` unchanged.

## Objective

Remove the create-path double-fetch: `create_hnsw_index` already materializes every base tuple, then each `hnsw_put` `ensure_key` loads vectors again via `store_tx.get`. Warm a create-scoped cache (or rebuild-only columnar buffer) **before** `hnsw_search_level` so Track 009’s parallel `v_dist` is not starved on SQLite decode.

Product north star: Ledgerful create on SQLite is get+put heavy. If 021 shows I/O/decode, SIMD (022) will not move wall clock.

## Plan-time snapshot (2026-09-02) — re-verify at execute

- `relation.rs` `create_hnsw_index`: `scan_all` → `TempCollector` then sequential `hnsw_put`.
- `hnsw.rs` `hnsw_put`: **new** `VectorCache` every call — so even decoded vectors from the scan are thrown away.
- `ensure_key` (~259–288): miss → `handle.get` → clone vector into cache.
- `hnsw_search_level`: sequential `ensure_key` for the neighbor batch, then parallel `v_dist` if rayon && batch ≥ 8.
- SQLite `get` is prepared-stmt + lock. Create does not **batch** puts (`SqliteTx::put` already delegates to `par_put`).
- RAM: 14k × 768 F32 ≈ **42 MiB** raw plus hashmap/`Vector`/`CompoundKey` overhead. That is **not** the measured bound — record peak RSS. F64 and dim 1536/3072 scale ~2–4×. `TempCollector` is `SwapVec` (can spill); a create-wide RAM cache duplicates vectors.
- Create runs under `lock.write()` + one write txn (`db.rs` ~1295–1301). Document that in Req 3; do not treat “WAL readers see pre-create” vs “writers blocked” as a product change.
- Incremental `hnsw_put` also lives in `query/stored.rs`. Create-scoped cache must not silently change that path.

## Spike / kill

**Kill if:** 021 shows distance math dominates wall clock (then 022, not this).

**Keep if:** I/O/decode share is large (plan-time: ≥25% of create) **and** a prefetch cut is measurable on the Ledgerful fixture.

## Requirements

1. Create-only path may use a create-scoped `VectorCache` (preferred: new `hnsw_put_with_cache` so `query/stored.rs` incremental puts stay untouched). Do not change on-disk layout. Migration = separate track + `BREAKING.md`.
2. **Document** peak RSS vs today’s per-put cache on the 14k fixture. A hard cap is **not** required to keep this spike; if a keep ships as the default create path, add a cap (or SwapVec-aware bound) **before** calling it done for large/WASM tables.
3. SQLite + MVCC: still one write txn; no torn index. Document live lock: exclusive write on the base relation for the sysop.
4. Metric for “prefetch win” is **`store_tx.get` count/ns**, not `ensure_key` call count (hits still call `ensure_key`).
5. Do **not** pre-`ensure_key` via `handle.get` on tuples just scanned. If warming: extract `DataValue::Vec` from `TempCollector` / rely on `vec_cache.insert` at insert (`hnsw.rs` ~351) once the cache is retained across puts.
6. Optional: retune `HNSW_PAR_DIST_THRESHOLD = 8` **after 022**, not as the first lever.
7. `compact-single-threaded` semantically unchanged.

## Out of scope

Bulk MVCC rewrite (**024**). SIMD (**022**). GPU. Incremental optimize (**025**).

## Dependencies

**021**. Optional after **022** (threshold retune only).

## §9 Deferred

None. Do not steal 024 bulk commit.

### Fold-in (2026-09-02)

| Id | Disposition | Action |
| :--- | :--- | :--- |
| opencode-M1 | **Agree — fold** | `stored.rs` in Files; prefer new create-only entry point. |
| opencode-m1 | **Agree — fold** | Metric = `store_tx.get`. |
| T023-F01 | **Agree — partial** | Document RSS on 14k; hard cap only if keep becomes default (not a Blocker for the spike). |
| T023-F02 | **Agree — fold** | Strike get-based pre-ensure_key. |
| T023-F03 | **Decline** | 50k–100k constrained-cache test is not this spike’s DoD. |
| T023-F04 | **Agree — fold** | Note SwapVec dual-retention. |
| T023-F05 | **Agree — fold** | F64 / other dims in RAM note. |
| T023-F06 | **Already covered** | Columnar only if hashmap loses. |
| T023-F07 | **Agree — fold** | Document exclusive write lock. |

## Last-PR Cursor comments

**N/A this track.** Empty GitHub PR scan (see 021).

## Tools (planning)

ledgerful + ai-brains used (vault empty). Live create/ensure_key path confirmed.

## Testing / Definition of Done

- [x] Create-time vs 021 baseline on the same fixture (`results.md`; 14k ignored release: 19.9 min → 12.6 min).
- [x] Compact-single-threaded unchanged semantically (`cargo check -p cozo --no-default-features --features compact-single-threaded`).
- [x] Kill or keep recorded (**KEEP**).
- [x] No format break without BREAKING.md.

## Hard locks

Public `::hnsw create` options remain valid. No `.unwrap()` in production (`TempCollector` already uses unwrap on the create path — do **not** add more; do not expand that pattern).
