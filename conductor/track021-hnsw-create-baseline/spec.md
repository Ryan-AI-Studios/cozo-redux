# Track 021: HNSW create baseline (cost model)

## Status

**Completed**

## Objective

Split `::hnsw create` wall clock on a Ledgerful-shaped rebuild into named buckets so later spikes can be kept or killed with numbers, not guesses. This track ships a **measurement**, not a speedup.

Product north star: Ledgerful git-deps this fork and does `::hnsw drop` + `::hnsw create` on large ingest batches. Every later create-speed track (022–028) is only justified if 021 shows that track’s bucket is the bottleneck.

## Plan-time snapshot (2026-09-02) — re-verify at execute

Live create path (not training memory):

```
SysOp::CreateVectorIndex
  → SessionTx::create_hnsw_index   cozo-core/src/runtime/relation.rs (~1020)
      scan_all → TempCollector (SwapVec)
      sequential hnsw_put per tuple
  → SessionTx::hnsw_put            cozo-core/src/runtime/hnsw.rs (~1030)
      fresh VectorCache per put (NOT create-wide)
      → hnsw_put_vector → hnsw_search_level + store_tx.put
```

| Claim | Plan-time fact |
| :--- | :--- |
| L2 | `let diff = a - b; diff.dot(&diff)` on `ndarray::Array1` (`hnsw.rs` ~193–200). Allocates. |
| `VectorCache` lifetime | **Per `hnsw_put`**, dropped at return. Does **not** retain every vector for the whole create. Within one insert it holds the new vector plus every `ensure_key` neighbor. |
| Double-fetch | True: full `scan_all` into `TempCollector`, then `ensure_key` → `RelationHandle::get` again. |
| Parallel dots | Sequential `ensure_key`, then `v_dist` parallel iff `feature = "rayon"` and `unvisited.len() >= HNSW_PAR_DIST_THRESHOLD` (8). Create passes `filter: None`, `pq_dist_table: None`. |
| Parser defaults | **None** for `m` / `ef_construction` — both must be set (`parse/sys.rs`). Distance default L2, dtype default F32. Manifest: `m_max = m`, `m_max0 = 2m`. |
| SQLite | Each graph edge is a SQL upsert on BLOB KV `cozo(k,v)` inside one write txn. `SqliteTx::put` **delegates to** `par_put` (`sqlite.rs` ~199–201) under a statement mutex. Create never **batches** puts and never uses caller-side parallel `par_put`. If 024 Option B is considered, the SQLite win is fewer statements, not “switch to `par_put`.” |
| Instrumentation | **None** on create. No HNSW benches. `cozo-core` has no `tracing` crate. |
| Ledgerful pin | `vector_store.rs` `rebuild_hnsw_index`: `::hnsw create snippet_embedding:snippet_idx {dim, dtype:F32, fields:[embedding], distance:L2, m:16, ef_construction:100}` on **SQLite**. Dim typically 768 (nomic). Rebuild when `batch_len >= semantic.hnsw_rebuild_threshold` (default 500). No `train_pq`. Git rev at plan-time: `14179d30` (bump independently). |

## Spike / kill

This track **is** the measurement. It does not ship a speedup.

- **Done (keep the table):** published ms/% table + written keep/kill for 022–028.
- **Kill later tracks** that 021 shows are not the bottleneck (see matrix below). Do not skip writing the table.

## Requirements

1. Repeatable fixture matching Ledgerful: SQLite, F32, dim **768**, ~**14k** rows (10k–20k acceptable), `m: 16`, `distance: L2`, `::hnsw drop` + `::hnsw create`. Run `ef_construction` **50** and **100**. Seeded **unit-normalized** 768-d F32 (Ledgerful normalizes before put). Deterministic seed.
2. Primary numbers on **Windows** (this machine). Optional Unix column if cheap.
3. Wall-clock buckets (ms and % of create):
   - `scan_all` / `TempCollector` materialize
   - `ensure_key` / `store_tx.get` / decode — time the **whole neighbor batch**, not per key
   - **intra-put** VectorCache miss vs hit (not create-wide cache quality). Record `VectorCache` instances created (may be < N if a tuple has no vectors)
   - `dist` / `v_dist` (L2) — time at **batch** level in `hnsw_search_level` (around the `distances` Vec), not per `Instant` inside `par_iter`
   - graph/heaps (`hnsw_search_level` minus dist); **do not** dump `tx.commit()` / `FlushFileBuffers` into this remainder
   - `store_tx.put` count + time
   - **`tx.commit()`** (or bound the table to `SessionTx::create_hnsw_index` and say so)
4. Instrumentation: env-gated `Instant` counters (e.g. `COZO_HNSW_CREATE_STATS=1`). **No** new default-on `tracing` dep. No per-`v_dist` `Instant` inside Rayon. No result-order or storage-format change. No `.unwrap()` / `.expect()` in production (including new counters).
5. Written keep/kill for 022–028 in this spec (fill the table after the run). 026 may run in parallel using the same fixture. **This matrix is the keep/kill authority** for 022–028.
6. `compact-single-threaded` still builds if any engine counters land: `cargo check -p cozo-core --no-default-features --features compact-single-threaded`.

## Keep / kill matrix (fill after the run)

Thresholds are plan-time; re-verify at execute. “Share” = that bucket’s % of create wall clock at `ef_construction: 100`, 14k×768, SQLite, Windows.

| Track | Keep if | Kill if | **021 result (ef100, 14k×768, SQLite, Windows, release)** |
| :--- | :--- | :--- | :--- |
| **022** | L2/`dist` share ≥ **25%** of create **or** alloc-free L2 ≥2× on the distance microbench | `put` + `ensure_key` together ≫ `dist` (dots will not move create) | **KILL** — dist **10.3%** < 25%; ensure_key+put **35.1%** ≫ dist. No L2 microbench (not needed for the 25% gate). |
| **023** | `ensure_key`/decode share ≥ **25%** | distance math dominates; prefetch cannot pay for itself | **KEEP** — ensure_key **34.2%**. |
| **024** | `put` share ≥ **30%** **or** put count per vector is huge vs graph CPU | 022+023 already close the gap; or HITL rejects durability tradeoff | **KILL** — put **0.94%** ≪ 30%. ~128 puts/vector but put wall ≪ graph CPU **54.6%**. |
| **025** | Incremental insert-all quality vs rebuild is in scope after 021 (time of drop+create is the pain Ledgerful feels) | 021 shows create is already cheap vs ingest/embed (out of Cozo) | **KEEP** — ~20 min create is product pain; ingest is out of Cozo. |
| **026** | Independent (config). Always run; 021 fixture is shared | N/A as a code spike | **KEEP** (always). |
| **027** | Only after 021; kill if `train_pq` ≥ full L2 create at 14k×768 | Training ≥ L2 create; or 021–023 already win | **deferred to 027 spike** — `train_pq` not run (not cheap). Not KEEP. |
| **028** | Only if 021+022 still leave a large gap **and** neighbor batches are large enough that GPU copy can amortize | Batches are `m`-sized (~16); PCIe/sync wins; 022 is enough | **KILL** — dist 10.3% does not dominate; mean batch **~20.8** keys, not ≫ m. |

## Out of scope

SIMD, GPU, PQ, CozoScript API changes, replacing HNSW, editing Ledgerful, changing Cozo defaults.

## Dependencies

None. **Must complete before** 022–028 *implementation* bets. 026 measurement may share this fixture and run in parallel. 029 is search-side and not blocked.

## §9 Deferred

| ID | Action | Notes |
| :--- | :--- | :--- |
| D-009-01 | **Decline** | Search-side; owner **029**. |
| D-012-01 | **Decline** | PQ construction; owner **027**. |
| D-HYG-01 | **N/A** | Closed when remediations landed on `main`. |

No related open lows to absorb. Silent skip would be a fail; nothing else in `docs/deferred.md` applies.

### Fold-in (2026-09-02)

Sources: `opencode-review.md`, `agy-review.md`. Stay **Ready — not started**.

| Id | Disposition | Action |
| :--- | :--- | :--- |
| agy-F-021-01 | **Agree — fold** | Batch-level dist/`ensure_key` timing; no `Instant` inside `par_iter`. |
| agy-F-021-02 | **Agree — fold** | Separate `tx.commit()` bucket or bound table to `create_hnsw_index`. |
| agy-F-021-03 | **Agree — fold** | Unit-normalized fixture. |
| agy-F-021-04 | **Agree — fold** | Explicit `compact-single-threaded` check command. |
| agy-F-021-05 / opencode-O2 | **Agree — fold** | Intra-put hit/miss; cache-instance count may be < N. |
| opencode-m1 | **Agree — fold** | SQLite `put` == `par_put` delegation; create does not batch. |
| agy-F-021-06 | **Already covered** | No new unwraps. Existing sqlite mutex unwraps are not this track. |
| opencode-O1 | **Already covered** | Re-verify line numbers at execute. |

## Last-PR Cursor comments

**N/A this track.** `gh pr list --repo Ryan-AI-Studios/cozo-redux --state merged --limit 4` returned `[]`. Repo `has_issues: false`. Old remote `UnlikelyKiller/cozo-redux` also has 0 PRs. Last product commits (`307ae144`, `128012d4`, `14179d30`, `acb20b24`) are remediations/docs, not HNSW create. Nothing to fold; no new placeholder **030**.

## Tools (planning)

- `ledgerful doctor --json`, `change-context --json --paths …`, `ledger status --compact`
- `ai-brains preflight --summary`, `sync query`, `recall --semantic --limit 5` — vault has **0** memories; discovery grants empty. Plan is from live code + Ledgerful call sites, not vault.
- Live: `hnsw.rs`, `relation.rs`, `parse/sys.rs`, `storage/sqlite.rs`, Ledgerful `src/semantic/vector_store.rs`

## Testing / Definition of Done

- [x] Cost table checked in under `conductor/track021-hnsw-create-baseline/` (or `docs/` if tracked).
- [x] VectorCache lifetime confirmed in the table (per-put vs create-wide).
- [x] Keep/kill row filled for 022–028; conductor notes updated if a track is killed at measurement time. (orchestrator owns `conductor.md`; this spec is the keep/kill authority.)
- [x] Fixture script/instructions in this track folder; seeded; Windows numbers recorded.
- [x] If counters land in `cozo-core`: fmt, clippy `-D warnings`, `nextest --lib --bins --workspace`; `compact-single-threaded` still builds.
- [x] No silent storage-format change.

## Hard locks

- Keep `::hnsw create { dim, dtype: F32, fields, distance: L2, m, ef_construction }` working.
- `cozo-core` owns engine; no FAISS/Qdrant default.
- miette + `Result`; no `.unwrap()` / `.expect()` in production.
