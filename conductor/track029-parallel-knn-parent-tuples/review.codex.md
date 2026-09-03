# Track Completion Audit — track029-parallel-knn-parent-tuples

## Verdict: PASS

Core DoD is met. Concurrent-read gate, SQLite sequential fallback, lazy chunked parent k-NN (including trailing partial chunks), `:limit`, `map_init` stacks, and no new `unsafe` for `&SessionTx` sharing all match `spec.md`. D-009-01 remains open until squash-merge, as specified.

P0: 0 | P1: 0 | P2: 0 | P3: 3

## Scope Reviewed

- Track dir: `conductor/track029-parallel-knn-parent-tuples/` (`spec.md`, `plan.md`, `results.md`, `internal-review.md`, `foldin-note.md`)
- `docs/deferred.md` D-009-01
- Working tree vs `origin/main` (`0e4f3e11`) on branch `track/029-parallel-knn-parent-tuples`
- Product: `cozo-core/src/query/ra.rs`, `cozo-core/src/storage/{mod,sqlite,mem,rocks,fjall,newrocks}.rs`, `cozo-core/src/runtime/mod.rs`, untracked `cozo-core/src/runtime/hnsw_parallel_knn_test.rs`
- Supporting reads: `SessionTx` (`transact.rs`), `hnsw_knn` (`hnsw.rs`), `QueryLimiter` (`eval.rs`), `Cargo.toml` features, sqlite statement mutex, tikv/temp (default false)
- Not re-run: workspace nextest / clippy / fmt (parent asked not to overlap cargo jobs). Compact-single-threaded compile not re-run.

Git product delta vs `origin/main`: `ra.rs` +132/−21-class parent iter rewrite; storage trait default + five overrides; test module wired; `hnsw.rs` unchanged (create-path rayon not touched).

## Requirement and DoD Matrix

| Requirement | Met/Partial/Unmet/Not verifiable | Evidence | Tests | Gap |
| :--- | :--- | :--- | :--- | :--- |
| 1. Absorb D-009-01: parallel parent k-NN + explicit concurrent-read gate | Met (row stays open until merge) | `StoreTx::is_concurrent_read_safe` default false; `HnswChunkedKnnIter` only when `rayon` + gate true | mem Reader vs Writer golden | Close D-009-01 only after squash-merge (`results.md`, `deferred.md`) |
| 2. No graph mutation races; `compact-single-threaded` sequential | Met | Search stays `&SessionTx` / `hnsw_knn(&self)`; chunked iter is `#[cfg(feature = "rayon")]`. `compact-single-threaded` does not enable `rayon`, so parent loop is the sequential `map_ok` path | Orchestrator: `cargo check -p cozo --no-default-features --features compact-single-threaded` | No runtime test under that feature (compile gate is the DoD) |
| 3. Correctness vs sequential (neighbors / scores within ulps) | Met (mem) | Immutable mem = Reader/parallel; `run_default` = Writer/sequential; neighbor ids equal; dist `< 1e-6` | `mem_parent_knn_parallel_matches_sequential_on_same_db` | fjall/rocks tests compare counts only (P3) |
| 4. Do not conflate with create-path rayon | Met | `hnsw.rs` not in the 029 diff; inner `v_dist` rayon remains 009 Phase 2 | — | — |
| 5. Filter stack: `map_init`, not `Mutex` | Met | `into_par_iter().map_init(Vec::new, …)`; no `Mutex` around stack in `ra.rs`. `eval_bytecode` `stack.clear()`s per eval | `mem_parent_knn_filter_even_ids` | Test does not compare filter results to sequential |
| 6. SQLite explicit false; do not keep a sqlite-true-only design | Met | `SqliteTx::is_concurrent_read_safe` returns `false`; `get` still mutexes cached statements; parallel iter never entered | `sqlite_parent_knn_works_without_concurrent_reads` (default `storage-sqlite`) | Test does not assert the flag (P3) |
| 7. Lazy bounded chunks; honor `:limit`; false backends sequential without error | Met | Chunk size 8; leftover `< 8` expanded sequentially then `pending.extend`; `QueryLimiter` stops pulling after N unique entry rows | `mem_parent_knn_limit_stops_at_five`; 12 parents ⇒ 8 parallel + 4 sequential remainder (36 rows) | Limit test checks cardinality, not that leftover parents were unpulled |
| Spike KEEP: real multi-parent workload; no `unsafe` share | Met | `results.md`: Ledgerful still single `$query_vec`; KEEP as generic Cozo `*queries, ~idx`. `&SessionTx` is `Send` via `SessionTx: Sync`; no new `unsafe` | Sharing compiles (clippy reported) | ≥2× speedup not measured (P3) |
| Backend matrix sqlite / rocks / fjall / mem | Met | sqlite false; mem Reader true / Writer false; rocks true; fjall true; newrocks false; tikv/temp inherit default false | sqlite+mem default; fjall/rocks `cfg` gated | Feature-gated tests not in the reported 198-test default nextest |
| DoD: workload note in `results.md` | Met | Ledgerful single-query; reframe documented | — | — |
| DoD: KEEP, no `unsafe` | Met | Phase 2 exit: no HITL `unsafe` | — | Pre-existing `unsafe impl Sync` on sqlite/rocks/newrocks unchanged |
| DoD: tests listed in spec | Met | mem golden + filter + limit + mutable + sqlite; fjall/rocks feature-gated; module in `runtime/mod.rs` | See Tests/Evidence | — |

## Findings

### [P3] SQLite sequential path is not pinned by an assertion on the gate

Confidence: High
Requirement: 6 (SQLite explicit false)
Location: `cozo-core\src\runtime\hnsw_parallel_knn_test.rs:183-190`; `cozo-core\src\storage\sqlite.rs:207-209`
Problem: The sqlite test only checks that 12×k rows return. It does not assert `is_concurrent_read_safe() == false`. A later flip of the sqlite override to `true` would still pass this test (mutex convoy, not a hard failure).
Evidence: Test body is `load_parent_knn_fixture` + `assert_eq!(rows.len(), QUERY_N * K)`. Implementation currently returns `false` and `HnswSearchRA::iter` therefore uses sequential `hnsw_expand_parent`.
Failure scenario: Regression sets sqlite concurrent-read true; Ledgerful search pays outer rayon + statement mutex without a red test.
Correction: Assert the flag on a sqlite read tx (or a tiny `StoreTx` unit test). Optional: fail if `HnswChunkedKnnIter` is constructed for sqlite.
Verification: `cargo nextest run -p cozo --lib sqlite_parent_knn`
Deferrable: Yes (implementation is already false; test hole only)

### [P3] fjall/rocks “sequential vs parallel” tests do not compare neighbors, and both legs can be parallel

Confidence: High
Requirement: 3 / backend matrix
Location: `cozo-core\src\runtime\hnsw_parallel_knn_test.rs:192-214`; `fjall.rs:107-109`; `rocks.rs:174-176`
Problem: Those tests only assert equal row counts. `FjallTx` and `RocksDbTx` always return `is_concurrent_read_safe() == true`, so `run_default` (write tx) still takes `HnswChunkedKnnIter`. The mem test is the real golden (Writer is false).
Evidence: No `neighbor_ids` / dist compare on fjall/rocks. Mem explicitly uses Immutable vs `run_default`.
Failure scenario: Backend-specific neighbor divergence would not fail these tests.
Correction: Compare sorted `(qid, id)` (and dist ulps) like the mem test; or force a sequential baseline via `compact-single-threaded` / a test-only sequential hook.
Verification: `cargo nextest run -p cozo --lib --features storage-fjall` and `--features storage-rocksdb`
Deferrable: Yes (mem golden covers the algorithm; these are smoke tests)

### [P3] KEEP recorded without a measured ≥2× vs sequential

Confidence: High
Requirement: Spike / kill keep criterion in `spec.md`
Location: `conductor\track029-parallel-knn-parent-tuples\results.md`; spec spike/kill paragraph
Problem: Spec keep gate is “real multi-parent workload **and** speedup ≥2×”. Workload reframe is documented; no timing table vs sequential on mem/rocks/fjall.
Evidence: `results.md` KEEP table has backends and chunk size 8, no wall-clock ratio.
Failure scenario: KEEP ships a parallel path that is correct but not a win on real parent-join sizes (still allowed by DoD checkboxes, which do not list 2×).
Correction: One measured parent-join batch (e.g. QUERY_N≥8 on mem Reader vs Writer) if product still wants the 2× bar.
Verification: Same fixture as `hnsw_parallel_knn_test` with a larger N, or document that 2× is not a merge gate.
Deferrable: Yes (DoD checkboxes are met without the number)

## Completeness Sweep

Searched changed product files and the new test module for `TODO`, `FIXME`, `XXX`, `HACK`, `placeholder`, `stub`, `unimplemented!`, `#[ignore]`.

- No placeholders in the 029 parent-iter / gate / test surface.
- No new `unsafe` in `ra.rs`. Pre-existing sqlite `unsafe impl Sync` + statement transmute unchanged; parallel path does not run on sqlite.
- No swallowed errors in `HnswChunkedKnnIter`: parent `Err` is returned; expand `Err` is returned.
- `hnsw.rs` create-path / `v_dist` rayon not claimed as 029 work.
- `compact-single-threaded` correctly omits feature `rayon`; `HnswChunkedKnnIter` is not compiled. (The `rayon` crate remains a non-optional dependency; eval-level `par_iter` on non-wasm is pre-existing and out of 029 scope.)

## Wiring and Regression Review

```text
~rel:idx / HnswSearchRA::iter
  → StoreTx::is_concurrent_read_safe()
      false (default, sqlite, newrocks, tikv, temp, mem Writer)
        → sequential hnsw_expand_parent (one stack)
      true (mem Reader, rocks, fjall) AND feature rayon
        → HnswChunkedKnnIter
            pull ≤8 parent tuples (lazy)
            if chunk == 8: rayon into_par_iter + map_init stacks
            else: sequential expand (trailing remainder)
            pending flatten, yield
  → SessionTx::hnsw_knn (&self, local VectorCache)
  → store_tx.get / range_scan
  → QueryLimiter on entry rule (stops after :limit unique rows)
```

Invariants:

- **Determinism:** Rayon `collect` on an indexed `Vec` preserves parent order; mem golden compares sorted `(qid, id)` and distances.
- **SQLite sequential:** Gate false ⇒ never constructs `HnswChunkedKnnIter`. `get` remains statement-mutexed.
- **Trailing partial chunk:** `None` from parent breaks the fill loop; non-empty `chunk.len() < 8` still expands and `pending.extend`. `QUERY_N=12` would fail the 36-row assert if the last 4 parents were dropped.
- **`:limit`:** `initial_rule_non_aggr_eval` pulls the iterator until `incr_and_should_stop`. First `next()` may expand a full chunk of 8 (documented extra work); limiter then drops the iterator, so remaining parents are not pulled.
- **No Mutex around filter stack:** `map_init(Vec::new, …)` only.
- **No unsafe SessionTx sharing:** `StoreTx: Sync` ⇒ `SessionTx: Sync` ⇒ `&SessionTx: Send`. Closure captures `&SessionTx`, not the owned tx. `StoreTx` is still not `Send`.
- **Mem Writer:** `matches!(self, MemTx::Reader(_))` keeps mutable `run_default` sequential — required for the golden.
- **Fjall true on write txs:** `transact` ignores write flag; `get` is `&self` on `BTreeMap` + fjall `Keyspace`. Spec asked for fjall true. Search does not `put` during `hnsw_knn`.
- **newrocks false:** Correct (pessimistic `rocksdb::Transaction`).
- **unwrap:** New production paths use `?` / `bail!`. No new production `unwrap`.
- **BREAKING.md:** No on-disk format change.
- **Windows paths:** Unused by this track.

## Verification Evidence

**Observed now (read-only):**

- Gate defaults and overrides match spec/results table.
- `HnswChunkedKnnIter` does not drop a short last chunk.
- `HNSW_PAR_PARENT_CHUNK == 8`; parallel only when `chunk.len() >= 8`.
- Tests exist and are wired in `runtime/mod.rs`.
- D-009-01 still open in `docs/deferred.md` with close-on-squash-merge note.

**Reported by orchestrator (not re-run):**

- `cargo fmt --all -- --check` pass
- `cargo clippy --all-targets --all-features -- -D warnings` pass
- `cargo nextest run --lib --bins --workspace`: 198 passed
- `cargo check -p cozo --no-default-features --features compact-single-threaded` pass

**Recommended:**

- `cargo nextest run -p cozo --lib hnsw_parallel_knn`
- `cargo nextest run -p cozo --lib --features storage-fjall` / `storage-rocksdb` (not in default 198)
- Optional: assert sqlite `is_concurrent_read_safe() == false`

**Not verifiable here:**

- ≥2× speedup
- fjall/rocks tests actually executed (compile covered by reported clippy `--all-features`)
- Runtime behavior under `compact-single-threaded` (compile-only)

## Deferred Candidates

Reviewer proposes only; orchestrator owns `docs/deferred.md`.

| Item | Why deferrable | Notes |
| :--- | :--- | :--- |
| P3 sqlite flag assertion | Easy; fix if touching tests | Not a product deferral if you add three lines |
| P3 fjall/rocks neighbor compare | Easy | Same |
| P3 ≥2× KEEP measurement | Spec spike bar, not a DoD checkbox | Do not close D-009-01 over this; owner already KEEP’d on workload reframe |

D-009-01 itself is **not** a deferral candidate to drop — spec says keep the row open until 029 squash-merges.

## Completion Decision

**PASS.** Implement KEEP as specified: concurrent-read trait, SQLite stays sequential, lazy chunks of 8 with sequential remainder, `:limit` via existing `QueryLimiter`, per-worker `map_init` stacks, no new `unsafe`. Mem Reader vs Writer is a real sequential-vs-parallel golden (12 parents exercises a full parallel chunk plus a 4-parent tail). Three P3s are test/measurement gaps, not unmet core DoD.

Do not close D-009-01 until squash-merge, per spec and `results.md`.
