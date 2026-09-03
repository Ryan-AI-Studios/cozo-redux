# Track 021 results — Windows HNSW create cost model

Measurement only. No speedup. No storage-format change.

## Run

| Item | Value |
| :--- | :--- |
| Host | Windows (`x86_64-pc-windows-msvc`) |
| Engine | SQLite tempfile |
| N × dim | **14000 × 768** F32, unit-normalized |
| Seed | `StdRng` **21768** |
| Index | `snippet_embedding:snippet_idx`, L2, `m: 16` |
| Profile | **`--release`** (see note) |
| Date | 2026-09-02 |

Debug `cargo test … hnsw_create_baseline_14k -- --ignored` was still inside the first create after **~29 min** (no `ef50` snapshot). It was stopped. The same ignored test was then run **`--release`**: ~2 min compile + **1842 s** wall for ingest + ef50 create + drop + ef100 create. That is still in spec range (N=14000) and under the ~45 min abort-to-10k gate for the release run.

Raw JSON: `conductor/track021-hnsw-create-baseline/raw/ef50.json`, `ef100.json`.

## VectorCache lifetime

**Per `hnsw_put`, not create-wide.** `cache_instances == 14000 == N` on both creates. Intra-put hit rate is high (ef100: 415,490,175 hits / 33,130,117 misses ≈ **92.6% hits**). A create-wide cache would show `cache_instances == 1` (or ≪ N). Confirmed: fresh `VectorCache` in `hnsw_put` only (search/remove constructors are not counted).

## Cost table (ms and % of `create_hnsw_index`)

Commit is **not** in the graph/heaps remainder. `%` is share of `create_total` (SessionTx::create_hnsw_index). `::hnsw create` wall ≈ create_total + commit.

### `ef_construction: 50`

| Bucket | ms | % of create | Notes |
| :--- | ---: | ---: | :--- |
| scan / TempCollector | 91.5 | 0.01% | |
| ensure_key (batch) | 238,047 | **37.0%** | 890,458 batches, 20,032,267 keys |
| dist / v_dist (batch) | 73,041 | 11.3% | 890,458 batches; no Instant in `par_iter` |
| graph / heaps remainder | 324,589 | **50.4%** | create − scan − ensure_key − dist − put |
| store_tx.put | 8,316 | 1.3% | **1,343,968** puts |
| **create total** | **644,083** | **100%** | ≈ 10.7 min |
| tx.commit | 277 | — | own bucket |
| VectorCache | — | — | 14,000 instances; 216,330,331 hits / 19,777,832 misses |
| hnsw_put loop | 643,935 | 99.98% | 14,000 puts; remainder vs create is scan + idx schema |

### `ef_construction: 100` (keep/kill authority)

| Bucket | ms | % of create | Notes |
| :--- | ---: | ---: | :--- |
| scan / TempCollector | 67.0 | 0.01% | |
| ensure_key (batch) | 407,587 | **34.2%** | 1,607,948 batches, 33,477,876 keys |
| dist / v_dist (batch) | 122,515 | 10.3% | 1,607,948 batches |
| graph / heaps remainder | 650,428 | **54.6%** | neighbor walk, heaps, key encode, `get`, shrink |
| store_tx.put | 11,259 | 0.94% | **1,785,790** puts (~128 / vector) |
| **create total** | **1,191,856** | **100%** | ≈ 19.9 min |
| tx.commit | 281 | — | own bucket |
| VectorCache | — | — | 14,000 instances; 415,490,175 hits / 33,130,117 misses |
| hnsw_put loop | 1,191,725 | 99.99% | 14,000 puts |

Mean neighbor-batch size at ef100: `ensure_key_keys / batches` ≈ **20.8** (m-sized; `m=16`, layer-0 `m_max0=32`).

## Keep / kill (022–028)

Filled in `spec.md`. Summary at **ef100, 14k×768, SQLite, Windows, release**:

| Track | Result | Share / reason |
| :--- | :--- | :--- |
| **022** SIMD L2 | **KILL** | dist **10.3%** < 25%; ensure_key+put **35.1%** ≫ dist. No alloc-free L2 microbench (would not change the 25% gate). |
| **023** prefetch / decode | **KEEP** | ensure_key **34.2%** ≥ 25%. |
| **024** put batching | **KILL** | put **0.94%** ≪ 30%. Count is huge (~128 puts/vector) but wall time is not vs graph CPU (54.6%). |
| **025** incremental vs rebuild | **KEEP** | ~20 min drop+create is the Ledgerful pain; ingest/embed is out of Cozo. Create is not cheap. |
| **026** knobs | **KEEP** | Independent; 021 fixture is shared. |
| **027** PQ | **deferred to 027 spike** | `train_pq` not run (not cheap vs this 20 min create). Not KEEP. |
| **028** GPU dist | **KILL** | dist does not dominate (10.3%); batches ≈ 20.8, not ≫ m. |

Largest unowned bucket: **graph/heaps remainder 54.6%** (not dist, not ensure_key, not put, not commit). No 022–028 spike targets it directly. Call that out when planning after 023.
