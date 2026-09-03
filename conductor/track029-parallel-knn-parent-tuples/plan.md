# Track 029 plan

## Status

**Completed.** Re-read track 009 spec/plan Phase 3 and `docs/deferred.md` D-009-01 at execute. Not blocked on 021.

## Phases (map to DoD)

### 1. Workload (DoD: keep/kill)

- Confirm Ledgerful remains single `$query_vec` (plan-time: yes).
- Search in-tree tests / docs for `~rel:idx` with a parent relation producing many tuples.
- If no real batch, **kill** after the workload note. Do **not** close D-009-01 in the 2026-09-02 fold-in. Owner call: kill-now vs reframe as generic Cozo parent-join batches (see `foldin-note.md`).

### 2. Concurrent-read gate (DoD: trait)

- Add `StoreTx::is_concurrent_read_safe(&self) -> bool` default `false`.
- Evaluate SQLite (`Mutex` on statements — likely false), RocksDB, fjall, mem. Do **not** keep a design that only helps if SQLite is true. Do not mark SQLite concurrent-safe in this fold-in.
- `SessionTx` **is Sync** (`StoreTx: Sync`; `Box<dyn StoreTx>` is Sync). `StoreTx` is **not** `Send`. Sharing `&SessionTx` across Rayon can be type-legal; SQLite concurrent `get` is still a mutex convoy.
- `SessionTx` is **not cloneable**. Do not plan on cloning read-only handles.
- **Phase 2 exit:** if sharing `&SessionTx` requires `unsafe`, **kill or HITL**.

### 3. Parallel iter (DoD: tests) — only on keep

- `HnswSearchRA::iter` is lazy — do **not** drain all parents. Bounded chunks; honor `:limit`.
- If concurrent-safe and parent count ≥ threshold, rayon over parent-tuple **chunks**.
- Per-worker filter stacks via rayon `map_init`, **not** a `Mutex` around one stack.
- Backend matrix: sqlite / rocks / fjall / mem. Backends with `is_concurrent_read_safe() == false` stay sequential without error.
- Golden: sequential vs parallel neighbors.

## Files (expected)

- `cozo-core/src/query/ra.rs` (`HnswSearchRA::iter`)
- `cozo-core/src/runtime/hnsw.rs` (`hnsw_knn`)
- `cozo-core/src/storage/mod.rs` + backend impls
- This track dir: `results.md` (workload note)

## Gate

fmt / clippy / tests. Sequential `compact-single-threaded`.

## Execute notes

Track 009 plan still says “Override to true in RocksDB and Sled”. **Sled is gone** — do not add a sled impl. SQLite is the Ledgerful backend and is the conservative `false`.
