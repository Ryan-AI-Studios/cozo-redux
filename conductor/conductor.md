# Conductor: CozoDB-redux

Master registry for development tracks. **This file is the live track list.** Do not treat `docs/status.md` (May 2026, still lists Track 012 as in progress) as current.

Consumer of this fork: **Ledgerful** (`C:\dev\ledgerful`) git-deps CozoDB-redux and rebuilds HNSW with `::hnsw drop` + `::hnsw create` on large batches (~14k F32 × dim 768, `m: 16`, `ef_construction: 100`, L2, SQLite).

## Active Tracks

HNSW / create-speed **spikes**. **021 keep/kill is the authority** for 022–028. **023** KEEP (create-wide cache; 14k create 19.9→12.6 min). **025** measured skip-drop: **KILL** (B2 recall@10 vs A = 0.54; 16.0 min > A 11.9 min). **026** measured knobs: **KILL** a faster Ledgerful preset (stay `m:16`, `ef_construction:100`). Next implement: **027**, then **029**. **027** still needs a `train_pq` vs L2 create measurement. Do not implement 022 / 024 / 028.

| Track ID | Status | Objective | Folder |
| :--- | :--- | :--- | :--- |
| **021** | Completed | Baseline: split `::hnsw create` wall clock. ef100: graph/heaps 54.6%, ensure_key 34.2%, dist 10.3%, put 0.94%, ~20 min | [track021-hnsw-create-baseline](track021-hnsw-create-baseline/) |
| **022** | Killed (021) | SIMD / wide CPU L2. Dist **10.3%** < 25%; ensure_key+put ≫ dist | [track022-simd-cpu-l2](track022-simd-cpu-l2/) |
| **023** | Completed | Prefetch / cache-fill. **KEEP** — 14k create 19.9→12.6 min; `store_get_count=0` | [track023-hnsw-prefetch](track023-hnsw-prefetch/) |
| **024** | Killed (021) | Bulk create / put batching. Put **0.94%** ≪ 30% (count is high; wall is not) | [track024-hnsw-bulk-create](track024-hnsw-bulk-create/) |
| **025** | Completed | Incremental index / `::hnsw optimize`. **KILL** skip-drop — B2 recall 0.54, 16.0 min > A 11.9 min | [track025-hnsw-incremental-optimize](track025-hnsw-incremental-optimize/) |
| **026** | Completed | Fast-build presets. **KILL** faster Ledgerful knobs; stay `m:16`, `ef_construction:100` | [track026-hnsw-fast-build-presets](track026-hnsw-fast-build-presets/) |
| **027** | Ready — not started | PQ on construction (`::hnsw train_pq`). **Not KEEP** — `train_pq` not measured in 021 | [track027-pq-on-construction](track027-pq-on-construction/) |
| **028** | Killed (021) | GPU distance oracle. Dist does not dominate; mean batch ~20.8, not ≫ m | [track028-gpu-distance-oracle](track028-gpu-distance-oracle/) |
| **029** | Ready — not started | Track 009 Phase 3: parallel KNN across parent tuples (`SessionTx::hnsw_knn`; **D-009-01**) | [track029-parallel-knn-parent-tuples](track029-parallel-knn-parent-tuples/) |

Engine file for 021–029: `cozo-core/src/runtime/hnsw.rs`. Create path: `create_hnsw_index` → per-tuple `hnsw_put`.

## Follow-ups (do not reopen as missing)

| Item | Notes |
| :--- | :--- |
| Post-Codex remediations | Landed on `main` (`128012d4`): fjall feature/API rename, pyo3, vendor graph. Working tree is clean. Not a new track. |
| Track 009 Phase 3 | Deferred on purpose; queued as **029** (Ready — not started). |
| Track 012 PQ gaps | In-tree: `::hnsw train_pq`, encode, approx L2. Not used on Ledgerful create. Re-rank, cosine, `hnsw_convert_to_pq` belong in **027**. |

## Completed Tracks

| Track ID | Objective | Landed |
| :--- | :--- | :--- |
| **025** | Incremental HNSW vs rebuild; KILL skip-drop; B2 recall 0.54, 16.0 min > A 11.9 min | *(SHA after squash)* |
| **023** | Prefetch / cache-fill; KEEP create-wide VectorCache; 19.9→12.6 min | `056d189a` (#3) |
| **026** | Fast-build presets; KILL faster Ledgerful knobs; stay m:16 ef_construction:100 | `0447d02a` (#2) |
| **021** | HNSW create baseline (cost model); keep/kill for 022–028 | `2710891b` (#1) |
| **020** | tikv-client Remediation — 0.3→0.4 + vendor + tonic 0.11 | `ddf8138d` |
| **019** | sled → fjall Storage Migration — unmaintained sled backend replaced; public feature is `storage-fjall` (`128012d4`) | `39d56a2e` |
| **018** | graph/graph_builder fxhash Elimination — vendor + rustc-hash swap | `788a47c2` |
| **017** | Serialization Dep Modernization — swapvec + fast2s | `bd01e83d` |
| **016** | jieba Dict Decoupling — remove include-flate / proc-macro-error2 build dep | `39f35ba5` |
| **015** | Quick Security Wins — pyo3 CVE, wee_alloc removal, cbindgen/atty | `04e47c4a` |
| **014** | HNSW Engine Hardening — Two-phase removal + Miette error propagation | `ca806062` |
| **013** | Dependency Transitivity — swapvec path dependency for downstream lz4_flex fix | `6690fdac` |
| **012** | Storage Scale — Vector quantization (Product Quantization). Search/create still exact L2 unless `train_pq` is used. | `c191691e` |
| **011** | HNSW Precision — In-loop predicate filtering with ef expansion. Construction still passes `filter: None`. | `2919397a` |
| **010** | HNSW Durability — Graph repair on deletion (re-link neighbors) | `897dddb5` |
| **009** | Search Performance — Parallel FTS sort + HNSW **batched** `v_dist` (threshold 8). Sequential `ensure_key` still runs first. Phase 3 → **029**. | `a262357c` |
| **008** | Storage Layer — TempStore write-buffer, ByteRange alloc elimination, sled/fjall range bounds |  |
| **007** | Query Execution — parallel joins, filter, unification (nested rayon accepted) |  |
| **006** | Memory Efficiency — DataValue shrinking + SmallVec Tuple |  |
| **005** | Security Infrastructure (Semgrep, Gitleaks, Pre-commit) |  |
| **004** | Serialization Overhaul (`bincode` -> `postcard`) |  |
| **003** | Platform Modernization (`instant` -> `web-time`) |  |
| **002** | Unmaintained Hygiene (`lazy_static`, `adler`, `fxhash`) |  |
| **001** | Infrastructure & Security Patches (`lz4_flex`, `tokio`) — *partial: patch not transitive* |  |

---
*Updated: 2026-09-02*
