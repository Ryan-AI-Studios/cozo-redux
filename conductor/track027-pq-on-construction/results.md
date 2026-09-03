# Track 027 results — PQ on construction vs `train_pq`

Windows `x86_64-pc-windows-msvc`, SQLite tempfile, 14k×768, seed **21768**, `m: 16`, `ef_construction: 100`. `train_pq` defaults: subspaces **8**, centroids **256**, samples **10000**. Raw: `raw/pq_14k.json`.

## Gate (a) — post-hoc `train_pq` vs L2 create

| Step | Wall |
| :--- | ---: |
| `::hnsw create` | **695,664 ms (~11.6 min)** |
| `::hnsw train_pq` | **20,202 ms (~20 s)** |

Post-hoc `train_pq` is cheap next to create. It is **not** a create-speed win (it adds ~20 s after an 11.6 min rebuild). Keep `train_pq` as the only PQ path. Killing (a) as a create-speed strategy does **not** kill construction-PQ by itself.

## Gate (b) — construction-PQ estimate vs dist share

Instrumented create (`COZO_HNSW_CREATE_STATS`): dist **101,177 ms (14.55%)**, graph/heaps **83.5%**, ensure_key **0.60%**.

| Estimate | Wall |
| :--- | ---: |
| Encode 14k (zero codebook, same MAC shape) | **1,343 ms** |
| LUT build × 50 queries | **4.6 ms** |

Encode+LUT **is** cheaper than dist (~1.3 s vs 101 s). Construction-PQ still **loses** the keep gate: dist is only **14.55%** of create (below a 25% “dominates create” bar), `k_dist` cannot use ADC, and 023 already removed the I/O wall. Saving all of dist would leave ~10 min of graph/heaps.

**KILL construction-PQ.** No opt-in create-time PQ. Default `pq: None` unchanged. No format change, no HITL.

## Always in-scope (landed)

| Item | Outcome |
| :--- | :--- |
| `train_pq` L2 guard | Cosine/IP indexes error. Tests: `hnsw_train_pq_rejects_cosine` |
| `num_centroids` `1..=256` | Parser + `hnsw_train_pq`. Test: `hnsw_train_pq_rejects_centroid_overflow` |
| Re-rank | Exact `v_dist` on PQ survivors; `bind_distance` matches squared L2. Test: `hnsw_pq_search_reranks_with_exact_l2` |
| Convert | **Won’t do** (no Ledgerful need). Does not close D-012-01 |

## Handshake

Ledgerful keeps exact-L2 `::hnsw create` and does not call `train_pq`. Optional post-create `train_pq` remains for non-Ledgerful L2 indexes.

## RAM

`train_pq` is not streaming: it keeps up to `samples` F32 vectors plus the full relation tuple list in memory, then encodes. Default 10k×768 samples ≈ 31 MiB payload; 14k relation tuples are additional. Codebook default ≈ 0.75 MiB. `compact-single-threaded` still compiles.
