# Track 021 plan

## Status

**Completed.** Line numbers re-verified at execute (`create_hnsw_index` ~1020, `hnsw_put` ~1051, `VectorCache` in `hnsw_put` only for instance count).

## Phases (map to DoD)

### 1. Fixture (DoD: seeded Ledgerful-shaped create)

- SQLite file (temp), relation with one `embedding:<F32; 768>` column.
- Insert N ∈ [10000, 20000] seeded **unit-normalized** F32 rows (prefer **14000**).
- `::hnsw create { dim: 768, dtype: F32, fields: [embedding], distance: L2, m: 16, ef_construction: 50 }` and again with `ef_construction: 100`.
- Prefer `cozo-bin` / a small `cozo` crate test under this track dir — not a one-off REPL. Share with **026**.

### 2. Counters (DoD: buckets)

- Env-gated `Instant` accumulators on:
  - `create_hnsw_index` scan/materialize
  - `ensure_key` over each **unvisited batch** (count + ns), not per key
  - `dist` / `v_dist` around the **batch** `distances` Vec (never inside `par_iter`)
  - `store_tx.put` (count + ns)
  - remainder = graph/heaps (must not include commit I/O)
  - `tx.commit()` if the script times wall clock of `::hnsw create`
- Dump a single stderr/JSON line when `COZO_HNSW_CREATE_STATS=1`. Default off. No `tracing` crate unless it is already a workspace dep at execute (plan-time: it is not).
- Confirm VectorCache is constructed inside `hnsw_put` and dropped after. Label hit/miss **intra-put**. Cache-instance count may be < tuple count when a row has no vectors (Ledgerful fixture: every row has `embedding`).

### 3. Publish (DoD: table + keep/kill)

- Write `conductor/track021-hnsw-create-baseline/results.md` (gitignored folder: `git add -f` when committing).
- Fill the keep/kill matrix in `spec.md`.
- Update `conductor/conductor.md` if a later spike is killed at measurement time (do not implement 022–028 in this track).

## Files (expected)

| Read | Write |
| :--- | :--- |
| `cozo-core/src/runtime/hnsw.rs` | Counters only if needed, behind env gate |
| `cozo-core/src/runtime/relation.rs` (`create_hnsw_index`) | Same |
| `cozo-core/src/parse/sys.rs` | Read (no default changes) |
| `cozo-core/src/storage/sqlite.rs` | Read |
| This track dir | Fixture + `results.md` |

Prefer **no** product behavior change. Counters that cannot be env-gated stay out of production.

## Gate

If any `cozo-core` code lands:

```
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --lib --bins --workspace
```

`compact-single-threaded` still builds:

```
cargo check -p cozo-core --no-default-features --features compact-single-threaded
```

Fixture-only / markdown-only: skip engine clippy.

## Execute notes

- Do not overlap `cargo` / `ledgerful verify` jobs on Windows.
- Ledgerful is the consumer, not the bench harness — do not edit `C:\dev\ledgerful` unless asked.
- Re-verify: `HNSW_PAR_DIST_THRESHOLD`, `hnsw_put` cache construction, SQLite `put` path.
