# Track 025 results — incremental HNSW vs drop+create

Measurement only. No `::hnsw optimize` sysop. No storage-format change. Ledgerful not edited.

## Keep / kill

**KILL** skipping Ledgerful drop+create above `hnsw_rebuild_threshold`. Live-index appends (Build B2) miss the recall band **and** are not faster than a full rebuild at 14k.

Handshake: Ledgerful should **keep** drop + `::hnsw create` when a batch is ≥ 500. Incremental `:put` remains the path for small batches; this track does not claim those match rebuild quality at 14k. Periodic recreate is the documented policy. Do not add `::hnsw optimize`.

## Smoke (debug, N=48 × dim 8)

| Item | Value |
| :--- | ---: |
| N0 / batch | 24 then 3×8 live `:put` (`$data`, one script per batch) |
| recall@10 vs A | **1.0** |
| Puts inserted | asserted (node count +batch) |

## 14k × 768 (release, same fixture as 021)

Windows `x86_64-pc-windows-msvc`, SQLite tempfile, seed **21768**, `m: 16`, `ef_construction: 100`, query `ef: 100`, k=10, 50 queries (seed 21769). Raw: `raw/incremental_14k.json`.

B2 appends use one mutable `?[id, embedding] <- $data :put snippet_embedding {id, embedding}` per 500-row batch (Ledgerful cadence). Not 4,000 singleton `run_script` calls.

| Build | What | Wall |
| :--- | :--- | ---: |
| **A** | import 14k, one `::hnsw create` (Ledgerful rebuild) | **716,886 ms (~11.9 min)** |
| **B2** | import+create **10,000**, then **8×500** live `:put` | create 456,502 ms + append 503,925 ms = **960,427 ms (~16.0 min)** |

| Quality | Value |
| :--- | ---: |
| recall@10 vs **A’s neighbors** | **0.54** |
| Keep band | ≥ 0.90 |
| Keep? | **no** |

Puts actually inserted: each 500-row batch increased distinct layer-0 `fr_id` count by 500 (canary no-op did not fire). JSON: `put_mode=one_script_per_batch`, `rows_per_put_script=500`, `batches=8`.

B2 is **slower** than A (16.0 vs 11.9 min) because 10k create plus 4,000 incremental `hnsw_put` calls (in 8 write txns) is not cheaper than one 14k create after 023’s cache reuse.

## If-keep design note (not used)

Optimize was not kept. Reverse-link / tombstone / exclusive-write duration are N/A. Kill policy: **periodic recreate** (Ledgerful drop+create above threshold).

## Proposed API

None. Existing `::hnsw drop` + `::hnsw create` stays the rebuild. `:put` stays incremental for small batches.
