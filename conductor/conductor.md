# Conductor: CozoDB-redux

Master registry for development tracks. **This file is the live track list.** Do not treat `docs/status.md` (May 2026, still lists Track 012 as in progress) as current.

Consumer of this fork: **Ledgerful** (`C:\dev\ledgerful`) git-deps CozoDB-redux and rebuilds HNSW with `::hnsw drop` + `::hnsw create` on large batches (~14k F32 × dim 768, `m: 16`, `ef_construction: 100`, L2, SQLite).

## Active Tracks

HNSW / create-speed **spikes**. Folders exist as **Placeholder** (`spec.md` / `plan.md`). Not Ready — do not `/implement-track` until `/plan-track NNN` upgrades. Measure, kill or keep, then a real spec. Not a definition of done.

Suggested order: **021 → 026 (knobs) → 022 (alloc-free L2, then SIMD) → 023 if 021 shows I/O → 024/025 if put/graph-bound → 027/028 only with a measured gap → 029 search-side.**

| Track ID | Status | Objective | Folder |
| :--- | :--- | :--- | :--- |
| **021** | Placeholder | Baseline: split `::hnsw create` wall clock (ensure_key / SQLite decode, VectorCache, L2, graph/heaps, `put`) on a Ledgerful-shaped rebuild | [track021-hnsw-create-baseline](track021-hnsw-create-baseline/) |
| **022** | Placeholder | SIMD / wide CPU L2 (and cosine). First: alloc-free `&[f32]` L2 (ndarray `a - b` allocates). Then AVX2/NEON. No GPU. After **021**. | [track022-simd-cpu-l2](track022-simd-cpu-l2/) |
| **023** | Placeholder | Prefetch / cache-fill before Track 009 rayon. Create already scans all tuples then `ensure_key` fetches them again. Kill if 021 shows dots ≫ I/O. | [track023-hnsw-prefetch](track023-hnsw-prefetch/) |
| **024** | Placeholder | Bulk create vs per-row MVCC `hnsw_put`. Offline rebuild (lock, build graph, one commit) if create is put/graph-bound. | [track024-hnsw-bulk-create](track024-hnsw-bulk-create/) |
| **025** | Placeholder | Incremental index / `::hnsw optimize` so Ledgerful can append without drop+full create above `hnsw_rebuild_threshold` | [track025-hnsw-incremental-optimize](track025-hnsw-incremental-optimize/) |
| **026** | Placeholder | Fast-build presets (`ef` / `m`). Cheap if docs-only; Ledgerful hardcodes `m:16`, `ef_construction:100`. Parallel with 021. | [track026-hnsw-fast-build-presets](track026-hnsw-fast-build-presets/) |
| **027** | Placeholder | PQ on **construction** (Track 012 follow-on; **D-012-01**). Ledgerful never `train_pq`. Kill if training ≥ full L2 create at 14k×768. | [track027-pq-on-construction](track027-pq-on-construction/) |
| **028** | Placeholder | Optional GPU **distance oracle** (CPU default; SYCL/WebGPU, not CUDA). Park until 022+023 leave a measured gap. Neighbor batches are `m`-sized. | [track028-gpu-distance-oracle](track028-gpu-distance-oracle/) |
| **029** | Placeholder | Track 009 Phase 3: parallel KNN across parent tuples (`StoreTx` concurrent read; **D-009-01**). Search volume, not Ledgerful full rebuild. | [track029-parallel-knn-parent-tuples](track029-parallel-knn-parent-tuples/) |

Engine file for 021–029: `cozo-core/src/runtime/hnsw.rs`. Create path: `create_hnsw_index` → per-tuple `hnsw_put`.

## Follow-ups (do not reopen as missing)

| Item | Notes |
| :--- | :--- |
| Uncommitted remediations on `main` | Working tree still has post-Codex follow-up (fjall feature/API rename, pyo3, graph vendor, lockfile). Tracks 017–020 are **committed**; this is leftover hygiene, not a new track. |
| Track 009 Phase 3 | Deferred on purpose; queued as **029**. |
| Track 012 PQ gaps | In-tree: train/encode/approx L2. Not used on Ledgerful create. Re-rank, cosine, `hnsw_convert_to_pq` belong in **027** if at all. |

## Completed Tracks

| Track ID | Objective | Landed |
| :--- | :--- | :--- |
| **020** | tikv-client Remediation — 0.3→0.4 + vendor + tonic 0.11 | `ddf8138d` |
| **019** | sled → fjall Storage Migration — unmaintained sled backend replaced (HEAD still exposes `storage-sled` / `new_cozo_sled`; dirty tree renames toward `storage-fjall`) | `39d56a2e` |
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
