# Breaking Changes in CozoDB-redux

This document tracks breaking changes and migration requirements for users of the CozoDB-redux fork.

## v0.8.2-redux — no new storage break

Tracks **021–029** do **not** change on-disk layout, CozoScript `::hnsw create` syntax, or the embeddable `Db` / `DbInstance` API in a way that requires a data migration.

**Behavior tightening** (only if you call `::hnsw train_pq`):

* Cosine indexes now error (search LUT is L2-only). Previously this could run and produce wrong distances.
* `centroids` outside `1..=256` now error at parse / train time.
* After PQ search, reported `bind_distance` is exact L2 re-rank, not ADC.

Default HNSW create and exact L2 search are unchanged. SQLite search remains sequential.

The 0.8.1-redux items below still apply when coming from upstream CozoDB or a `sled` database.

## v0.8.1-redux

## 1. Storage Migration: sled -> fjall (Track 019)

**Impact:** Data Incompatibility
**Feature:** `storage-fjall`

The unmaintained `sled` storage engine has been replaced with `fjall`, a modern LSM-based engine.

*   **Breaking Change:** The on-disk formats are fundamentally different and incompatible. Existing `sled` database files **cannot** be opened by v0.8.1-redux.
*   **Migration Path:**
    1.  Using a v0.8.0-redux build, export your data to a backup (e.g., JSON or SQLite).
    2.  Upgrade to v0.8.1-redux.
    3.  Import the backup into a new `fjall`-backed instance.

## 2. Python Bindings: pyo3 0.29 Upgrade (Track 015)

**Impact:** API / Compilation
**Crate:** `cozo-lib-python`

The Python bridge has been upgraded to `pyo3` 0.29.x to resolve security advisories (RUSTSEC-2026-0176, RUSTSEC-2026-0177).

*   **Breaking Change:**
    *   Significant API changes between `pyo3` 0.20 and 0.29.
    *   Dropped support for `abi3-py37`. Minimum supported Python version for `abi3` is now 3.8.
*   **Remediation:** Ensure your environment is compatible with the latest `pyo3` standards and use Python 3.8+.

## 3. Distributed Storage: tikv-client 0.4.0 & tonic 0.12 (Track 020)

**Impact:** API / Configuration
**Feature:** `storage-tikv`

To resolve multiple security vulnerabilities in the gRPC and TLS stacks, `tikv-client` has been upgraded and vendored with a `tonic` 0.12 patch.

*   **Breaking Change:**
    *   `tonic` 0.12 moved several core types (e.g., `NamedService`) and updated its internal TLS implementation to `rustls` 0.24.2.
    *   Manual transport configurations or security managers utilizing the `tikv-client` API directly will require updates.
*   **Remediation:** Update any code interacting directly with `tikv-client` or `tonic` transport layers to match the 0.4.0 and 0.12 APIs respectively.

## 4. Internal Data Serialization (Track 017)

**Impact:** Temporary Data Incompatibility
**Crates:** `swapvec`, `fast2s`

Ongoing migration from `bincode` to `postcard` for internal auxiliary data.

*   **Breaking Change:** Disk-spilled data from `swapvec` (used for large result sets) may be incompatible between versions.
*   **Remediation:** Ensure all active transactions are committed and temporary files are cleared before upgrading.
