# CozoDB-redux Release Notes

## v0.8.1-redux

This release focuses on critical security remediations, storage backend modernization, and dependency decoupling across 6 major development tracks.

### Security Remediations

* **pyo3 Security Update** (Track 015) — Resolved OOB read vulnerabilities in `PyList` and `PyTuple` iterators by upgrading to pyo3 0.29.x.
* **tikv-client Hardening** (Track 020) — Upgraded and vendored `tikv-client` 0.4.0 with a patched `tonic` 0.11 stack. Resolved 6 security advisories in the `storage-tikv` path including `protobuf` recursion crashes, `rand` unsoundness, and multiple `rustls-webpki` CVEs.
* **Unsafe Allocator Removal** (Track 015) — Removed `wee_alloc` and other unmaintained allocator crates to improve system stability and security.
* **Modernized Crypto & TLS** — Swapped `atty` for `is-terminal` and upgraded `cbindgen` to resolve secondary audit findings.

### Storage & Dependency Modernization

* **sled → fjall Migration** (Track 019) — Replaced the unmaintained `sled` storage engine with `fjall`, a modern LSM-based engine, improving stability and performance for persistent key-value storage.
* **Serialization Overhaul in swapvec & fast2s** (Track 017) — Fully migrated from `bincode` (unmaintained) to `postcard` for internal data-value serialization in auxiliary storage crates.
* **jieba-rs Decoupling** (Track 016) — Removed `include-flate` and `proc-macro-error2` build dependencies by externalizing the `jieba` dictionary loading, significantly speeding up build times and reducing binary footprint.
* **fxhash Elimination** (Track 018) — Removed unmaintained `fxhash` and `graph` vendor dependencies in favor of `rustc-hash` for deterministic, high-performance hashing in graph algorithms.

### Infrastructure

* **ChangeGuard Integration** — Hardened the CI gate with ChangeGuard-verified behavior and provenance tracking for all security-sensitive edits.

## v0.8.0-redux

This is the consolidated release of the CozoDB-redux fork, incorporating 14 major development tracks for performance, security, and HNSW stability.

### New features

* **Product Quantization for HNSW** (`::hnsw train_pq`) — train codebooks on existing indexes to reduce vector storage and speed up approximate search.
* **In-loop predicate filtering for HNSW** — `filter:` clauses in HNSW queries now use biased traversal with `ef` expansion for correct K-results.

### Bug fixes

* **HNSW Engine Hardening** — Two-phase removal logic ensures no stale edges remain in the graph after deletion, preventing "key not found" panics.
* **Safe Error Propagation** — Replaced internal `unwrap()` and `expect()` calls with `miette`-based `Result` propagation across HNSW and storage layers.
* **HNSW graph repair on deletion** — Former neighbors whose degree drops too low are automatically reconnected via heuristic candidate selection.
* **HNSW metadata deserialization safety** — Added `decode_metadata` helper; all metadata deserialization paths now guard against short buffers, preventing index-out-of-bounds panics during semantic indexing.

### Performance & Efficiency

* **Parallel query execution** — Parallel iterators for joins, filters, unification, and FTS scoring.
* **Memory Efficiency** — `DataValue` shrinking and `SmallVec`-backed `Tuple` implementation to reduce heap overhead.
* **Allocation-free storage** — Elimination of `to_vec()` allocations in range scans across `MemStorage`, `TempStorage`, and `sled`.
* **TempStore write-buffer** — Optimized write path for temporary relations.

### Infrastructure & Security

* **Security Guardrails** — Automated `gitleaks`, `semgrep`, and `pre-commit` infrastructure.
* **Modernized Dependencies** — Migrated to `web-time`, `postcard`, and patched `lz4_flex`/`tokio` vulnerabilities.
* **Clean Hygiene** — Removed unmaintained/deprecated crates (`lazy_static`, `adler`, `fxhash`).

### Compatibility

* Preserves all upstream CozoScript syntax.
* Backward-compatible `HnswIndexManifest` loading.
* Full test suite validation (246 tests).

