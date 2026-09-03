# Track 023 results — create-scoped VectorCache

Prefetch is the default `::hnsw create` path. Incremental `hnsw_put` is unchanged (fresh cache). No storage-format change.

## Keep / kill

**KEEP** as the default create path.

021: `ensure_key` **34.2%** of create, `cache_instances == 14000`. After a create-wide HashMap cache, `store_tx.get` on `ensure_key` misses is **0**; neighbor vectors come from `vec_cache.insert` at insert. HashMap `CompoundKey → Vector` is enough; no columnar buffer.

## 14k × 768 (release, same fixture as 021)

Windows `x86_64-pc-windows-msvc`, SQLite tempfile, seed **21768**, `m: 16`, `ef_construction: 100`, unit-normalized F32. Raw: `raw/prefetch_ef100.json`. Ignored test `hnsw_prefetch_14k_vs_021` wall **755.8 s** (includes ingest + create).

| Item | 021 ef100 | 023 ef100 |
| :--- | ---: | ---: |
| `create_total_ms` | 1,191,856 (~19.9 min) | **755,102 (~12.6 min)** |
| `ensure_key_ms` / pct | 407,587 / **34.2%** | **4,372 / 0.58%** |
| `store_get_count` | ≈ 33,130,117 cache misses | **0** |
| `cache_instances` | 14,000 | **1** |
| `cache_misses` | 33,130,117 | **0** |
| `cache_peak` | (per-put) | **14,000** |
| dist ms / pct | 122,515 / 10.3% | 117,004 / 15.5% |
| graph/heaps ms / pct | 650,428 / 54.6% | 623,172 / **82.5%** |
| `store_tx.put` ms / pct | 11,259 / 0.94% | 10,487 / 1.39% |
| commit ms | 281 | 290 |

Create wall cut **~37%** (1.58×). `ensure_key` bucket dropped **99%**. Graph/heaps absolute time is almost unchanged (~623 s vs 650 s) — that remainder is **D-021-01**.

### RAM (peak working set of the test process)

| Item | Bytes |
| :--- | ---: |
| Before import | 5,619,712 (~5.4 MiB) |
| Peak working set | 124,383,232 (~118.6 MiB) |
| Delta | 118,763,520 (~**113 MiB**) |

Raw F32 payload 14k × 768 × 4 ≈ **42 MiB**. The rest is HashMap/`CompoundKey`/`Vector` overhead, SQLite pages, and ndarray. F64 or dim 1536/3072 scale ~2–4×. `TempCollector` (`SwapVec`) can spill; the create-wide HashMap **duplicates** vectors in RAM until create ends. Per-put cache was O(neighbor-batch), not O(N). Hard cap for large/WASM tables is **D-023-01**.

## Smoke (debug, N=48 × dim 8)

| Item | Value |
| :--- | ---: |
| `cache_instances` | **1** |
| `store_get_count` | **0** (= `cache_misses`) |
| Incremental `:put` | `cache_instances == 1` (fresh cache) |

## MVCC

Create still runs under one write txn (`run_sys_op` → `transact_write`) with `lock.write()` on the base relation (`db.rs` CreateVectorIndex). No torn index. WAL readers vs writers blocked is unchanged.

## Columnar vs hashmap

HashMap stays. `store_get_count == 0` means decode is gone; remaining `ensure_key` time is HashMap lookup (14k: 0.58%). Columnar translation would not beat that for this spike.
