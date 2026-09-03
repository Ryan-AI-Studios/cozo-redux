# Track 029: Parallel k-NN across parent tuples (search)

## Status

**In progress.** Owns **D-009-01**. Independent of create-speed 021–028.

## Objective

Finish Track 009 Phase 3: parallelize **search** over parent tuples in `HnswSearchRA::iter`, not graph mutation and not create-path rayon (already shipped).

Product north star: this is **search volume**, not Ledgerful full rebuild. Ledgerful `query_candidates` is a **single** `$query_vec` today — the keep gate must prove a real multi-parent or multi-query workload (Cozo RA, or a future Ledgerful batch) before paying `StoreTx` concurrent-read work.

## Plan-time snapshot (2026-09-02) — re-verify at execute

| Placeholder claim | Live |
| :--- | :--- |
| `HnswIndex::knn` is `&self` | **No such type.** Live: `SessionTx::hnsw_knn(&self, …)` (`hnsw.rs` ~1439). Intent (immutable search) holds. |
| Parent loop sequential | **True.** `HnswSearchRA::iter` (`query/ra.rs` ~1171–1206) `map_ok` → one `hnsw_knn` per parent row. The iterator is **lazy** — do not drain all parents. |
| Shared filter stack | `&mut stack` for filter bytecode — extra parallelization hurdle. If keep: per-worker stacks via rayon `map_init`, **not** a `Mutex` around one stack. |
| `StoreTx::is_concurrent_read_safe` | **Absent.** Trait is already `Sync` (`storage/mod.rs`). Track 009 plan still lists adding the method (default false; RocksDB/Sled true). **Sled is gone** — map to SQLite (Ledgerful) + RocksDB + fjall. SQLite default stays **false**. |
| `SessionTx` Sync | **Is Sync.** `StoreTx: Sync`, so `Box<dyn StoreTx>` is Sync. `StoreTx` is **not** `Send`. Sharing `&SessionTx` across Rayon can be type-legal; SQLite concurrent `get` is still a mutex convoy. Do **not** say “SessionTx is not Sync.” `SessionTx` is **not cloneable** — do not plan on cloning read-only handles. |
| Ledgerful | One query vector per `query` / `query_scoped`. Overfetch `k*10`. Not batched parent tuples. |
| `compact-single-threaded` | No rayon feature → inner `v_dist` sequential; parent loop sequential in **all** modes today. |

Track 009 original target (“batch semantic-impact scans, 50+ files”) is **not** the live Ledgerful HNSW query shape. Do not implement 029 solely because 009’s spec mentioned ChangeGuard.

## Spike / kill

**Keep if:** multi-query / parent-tuple batches are a real Cozo or Ledgerful workload **and** speedup is ≥2× with correct results vs sequential, including SQLite behavior. Do **not** keep a design that only helps if SQLite `is_concurrent_read_safe` is true.

**Kill if:** Ledgerful (and in-tree tests) are almost always single-query; then document and leave sequential. Kill SQLite parallel if `get` mutex makes outer rayon slower or unsafe. If sharing `&SessionTx` requires `unsafe`, **kill or HITL**.

Phase 1 workload stays: Ledgerful is single-query at plan-time. This fold-in does **not** close D-009-01. Owner call (see `foldin-note.md`): kill-now vs reframe as generic Cozo parent-join batches.

Do not treat invented “2–5× worse” latency numbers as facts. Measure if execute reaches a SQLite parallel attempt.

## Requirements

1. Absorb **D-009-01**: parallel k-NN over parent tuples with an explicit concurrent-read gate.
2. No graph mutation races. `compact-single-threaded` stays sequential (no rayon).
3. Correctness vs current sequential eval (same neighbors / scores within ulps).
4. Do not conflate with create-path rayon (009 Phase 2).
5. Shared filter `stack`: per-worker stacks via `map_init`, or no filter in the parallel path.
6. SQLite: default **false** for concurrent read until proven; do not assume Sled notes still apply. Do not mark SQLite concurrent-safe in this fold-in.
7. If keep: `HnswSearchRA::iter` stays lazy — bounded chunks; honor `:limit`. Backend matrix: sqlite / rocks / fjall / mem; `false` backends sequential without error.

## Out of scope

GPU. Create-path SIMD (**022**). Bulk create (**024**). Editing Ledgerful.

## Dependencies

Track **009** (shipped). **Not blocked on 021.**

## §9 Deferred

| ID | Action | Notes |
| :--- | :--- | :--- |
| **D-009-01** | **Absorb** | This track. Kill/keep may close the row later **after** the Phase 1 workload note. This fold-in does **not** close it. |
| D-012-01 | Decline | Owner **027**. |

### Fold-in (2026-09-02)

| Id | Source | Sev | Disposition | Action |
| :--- | :--- | :--- | :--- | :--- |
| SessionTx Sync | OpenCode M1 vs Agy | M | Agree — Agy (corrected) | `SessionTx` **is** Sync. `StoreTx` is not Send. Sharing `&SessionTx` can be type-legal; SQLite `get` is still a mutex convoy. Do not say “SessionTx is not Sync.” |
| opencode-M1 hatch | OpenCode | M | Agree — partial | Keep Phase 2 exit: if sharing requires `unsafe`, kill or HITL (intent kept; Sync typing claim was wrong). |
| agy-T029-M02 | Agy | M | Agree — fold | Strike “cloning read-only handles”. Not cloneable. |
| SQLite default false | Spec / agy-B01 | B | Already covered | Do not keep a design that only helps if SQLite is true. Do not mark SQLite concurrent-safe. |
| agy-T029-M01 | Agy | M | Agree — fold (if keep) | Iter is lazy; do not drain all parents. Bounded chunks; honor `:limit`. |
| agy-T029-m01 | Agy | m | Agree — fold (if keep) | Per-worker stacks via `map_init`, not `Mutex` around one stack. |
| agy-T029-B02 | Agy | B | Agree — fold (if keep) | Backend matrix: sqlite/rocks/fjall/mem; `false` backends sequential without error. |
| Phase 1 / D-009-01 close now | Agy m02 / plan | — | Decline | Workload Phase 1 stays. Do not close D-009-01 in this fold-in. Escalate: kill-now vs reframe as generic Cozo parent-join batches. |
| Invented 2–5× worse | Agy B01 | — | Decline | Not a measured fact. |

## Last-PR Cursor comments

**N/A this track.** Empty GitHub PR scan (see 021). Track 009 Phase 3 checklist is the source, not a GitHub review comment.

## Tools (planning)

ledgerful + ai-brains used (vault empty). Live `hnsw_knn` + `HnswSearchRA::iter` + Ledgerful query confirmed.

## Testing / Definition of Done

- [x] Workload note: Ledgerful still single `$query_vec`. Reframed as generic Cozo parent-join (`results.md`).
- [x] KEEP. Sharing `&SessionTx` needs no `unsafe`.
- [x] Tests vs sequential (mem Reader vs Writer) + `compact-single-threaded`; SQLite explicit false; lazy chunks honor `:limit`; `map_init` stacks; sqlite/mem tests in default; fjall/rocks tests feature-gated.

## Hard locks

No `.unwrap()` in production. Unsafe raw-pointer sharing (009 note) needs a SAFETY comment and HITL if it appears.
