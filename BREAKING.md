# Breaking Changes in v0.8.1-redux

This document tracks breaking changes and migration requirements for users of the CozoDB-redux fork.

## 1. Storage Migration: sled -> fjall (Track 019)

**Impact:** Data Incompatibility
**Feature:** `storage-sled`

The unmaintained `sled` storage engine has been replaced with `fjall`, a modern LSM-based engine. 

*   **Breaking Change:** The on-disk formats are fundamentally different and incompatible. Existing `sled` database files **cannot** be opened by v0.8.1-redux.
*   **Migration Path:**
    1.  Using a v0.8.0-redux build, export your data to a backup (e.g., JSON or SQLite).
    2.  Upgrade to v0.8.1-redux.
    3.  Import the backup into a new `fjall`-backed instance.

## 2. Python Bindings: pyo3 0.24 Upgrade (Track 015)

**Impact:** API / Compilation
**Crate:** `cozo-lib-python`

The Python bridge has been upgraded to `pyo3` 0.24.x to resolve security advisories (RUSTSEC-2026-0176).

*   **Breaking Change:** If you consume `cozo-lib-python` as a Rust dependency or rely on specific internal `PyO3` behaviors, you may encounter breaking changes in the Rust-Python interface.
*   **Remediation:** Ensure your environment is compatible with the latest `pyo3` standards.

## 3. Distributed Storage: tikv-client 0.4.0 & tonic 0.11 (Track 020)

**Impact:** API / Configuration
**Feature:** `storage-tikv`

To resolve multiple security vulnerabilities in the gRPC and TLS stacks, `tikv-client` has been upgraded and vendored with a `tonic` 0.11 patch.

*   **Breaking Change:** 
    *   `tonic` 0.11 moved several core types (e.g., `NamedService`) and updated its internal TLS implementation to `rustls` 0.22.
    *   Manual transport configurations or security managers utilizing the `tikv-client` API directly will require updates.
*   **Remediation:** Update any code interacting directly with `tikv-client` or `tonic` transport layers to match the 0.4.0 and 0.11 APIs respectively.

## 4. Internal Data Serialization (Track 017)

**Impact:** Temporary Data Incompatibility
**Crates:** `swapvec`, `fast2s`

Ongoing migration from `bincode` to `postcard` for internal auxiliary data.

*   **Breaking Change:** Disk-spilled data from `swapvec` (used for large result sets) may be incompatible between versions.
*   **Remediation:** Ensure all active transactions are committed and temporary files are cleared before upgrading.
