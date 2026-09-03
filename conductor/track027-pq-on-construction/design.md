# Track 027 design — PQ on construction vs post-hoc `train_pq`

## Status

**KILL construction-PQ.** `train_pq` remains post-create only. Search-side D-012-01 items in this PR: L2 guard, centroid bound, exact re-rank. Convert won’t-do.

## Convert (`hnsw_convert_to_pq`)

**Won’t do.** No symbol exists. Ledgerful never needs a live-index convert. Does **not** close D-012-01.

## Construction-PQ

Gate **(a):** 14k create **695.7 s**, `train_pq` **20.2 s**. Post-hoc train is cheap and is not a create-speed substitute.

Gate **(b):** dist **14.55%** of create (101 s). Encode estimate **1.3 s**, LUT **4.6 ms**. `k_dist` stays full-vector. Dist does not dominate create (graph/heaps 83.5%). No opt-in create-time PQ.

## Always in-scope (landed)

- `train_pq` requires `manifest.distance == L2`.
- `num_centroids` `1..=256` in the parser and in `hnsw_train_pq`.
- After PQ search, re-rank survivors with exact `v_dist`. `bind_distance` is exact.

## Default create

Unchanged: `HnswIndexManifest.pq: None`, exact `v_dist` during `::hnsw create`. No on-disk layout change. No GPU.

## RAM (`::hnsw train_pq`)

`hnsw_train_pq` materializes `all_samples` (up to `samples` F32 vectors × dim) and then `all_tuples` for the whole base relation before encoding. Default 10k samples × 768 F32 is ~31 MiB of sample payload; the tuple scan holds the full relation in RAM on top of that. Codebook is `subspaces × centroids × (dim/subspaces)` F32 (default 8×256×96 ≈ 0.75 MiB). Not streaming. `compact-single-threaded` still builds (no rayon on this path).
