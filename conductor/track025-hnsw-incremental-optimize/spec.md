# Track 025: Incremental HNSW / `::hnsw optimize`

## Status

**KILL.** Live B2 appends (8×500 `$data` `:put`) recall@10 vs A = 0.54 at 14k; B2 wall 16.0 min > A 11.9 min. Ledgerful keeps drop+create.

## Objective

Give Ledgerful a way to avoid `::hnsw drop` + full `create` when a batch exceeds `hnsw_rebuild_threshold` (default **500**). Either prove mixed live-index appends (Build B2) match rebuild recall at ~14k well enough that Ledgerful can stop dropping, or add an explicit `::hnsw optimize` / compaction sysop.

Product north star: large ingest should not pay a full rebuild if incremental graph quality is within an agreed recall band.

## Plan-time snapshot (2026-09-02) — re-verify at execute

Ledgerful (`C:\dev\ledgerful\src\semantic\vector_store.rs`):

| Batch size | Behavior |
| :--- | :--- |
| `>= hnsw_rebuild_threshold` (500) | Drop index, `:put` rows, `rebuild_hnsw_index` (drop+create over **whole** relation) |
| `< threshold` | Keep index; `:put` → incremental `hnsw_put` via live index |
| `disable_hnsw` | No HNSW |

- Track **010** only repairs **deletes** (re-link neighbors). It is not optimize-on-append.
- Incremental insert path already exists (`stored.rs` `update_in_hnsw` → `hnsw_put`). `create_hnsw_index` **is** a loop of `hnsw_put` (`relation.rs` ~1187–1196). Same-order “insert all into empty then no rebuild” vs create is **tautological** — do not use it as the quality test.
- `hnsw_put_vector` canary-hash early-return (`hnsw.rs` ~359–363) can no-op duplicate puts; quality runs must **assert puts actually insert**.
- `hnsw_shrink_neighbour` shrinks **outgoing** neighbour sets only. Reverse/in-degree links are a separate write. Tombstones are `ignore_link=true` (or `del` if already ignored).
- No `::hnsw optimize` sysop at plan-time (`parse/sys.rs`).
- Handshake: Ledgerful would have to stop drop+create — **out of scope to edit Ledgerful / `HnswRefreshPlan` unless asked**.

## Spike / kill

**Keep if:** append or optimize matches rebuild recall@k within the band below at much lower wall clock, **and** a named CozoScript API exists for Ledgerful to call.

**Kill if:** quality gap is large and Ledgerful must keep drop+create; document “always recreate above threshold”.

Recall band (plan-time): recall@10 vs **Build A’s neighbors** at query `ef: 100` ≥ **0.90** on the 14k×768 fixture. Optional brute-force column for absolute quality — do **not** replace vs-A. Re-verify at execute.

## Requirements

1. Quality vs time table:
   - **Build A:** `::hnsw drop` + `::hnsw create` over the 14k fixture (today’s Ledgerful rebuild).
   - **Build B2:** start from a **built** index, then mixed appends with a **live** index (e.g. 28×500, or N0=10k then Δ batches). This is the real Ledgerful cadence (`< threshold` keeps the index).
   - Do **not** use same-order empty insert-all as the quality test (that is create).
   - Primary recall: recall@10 vs **Build A’s neighbors** at query `ef: 100`. Optional brute-force column — do **not** replace vs-A.
   - Optional optimize pass only if keep.
2. Named CozoScript API if keep (`::hnsw optimize` or documented “do not drop”). No silent format break.
3. Two-sided handshake noted in spec; Ledgerful config / `HnswRefreshPlan` change is a follow-up, not this PR, unless the owner asks.
4. Keep public create options working.
5. If keep `::hnsw optimize`: design note must cover reverse-link / in-degree (shrink is outgoing-only), exclusive write-txn duration, tombstone compaction (`ignore_link`) + degree vs neighbour count under multi-batch put. If kill: document periodic recreate (Ledgerful keeps drop+create).

## Out of scope

GPU. PQ training (**027**). Bulk first create (**024**) except as a dependency note. Editing Ledgerful / `HnswRefreshPlan` (handshake note only). Nomic real embeddings (seeded 021-shaped fixture is enough). Gating this track on **023** (create also uses per-put `VectorCache`; 180s vs 35s / 2M lookups are unmeasured).

## Dependencies

**021**. Optional **024** (if bulk create changes the rebuild story, optimize may be unnecessary).

## §9 Deferred

None absorbed. D-009-01 → 029. D-012-01 → 027.

### Fold-in (2026-09-02)

| Id | Source | Disposition | Action |
| :--- | :--- | :--- | :--- |
| agy-F-025-01 | Antigravity | Agree | Rewrite Phase 1: Build A = drop+create; Build B2 = mixed appends on a live index. Same-order empty insert-all is tautological. |
| opencode-M1 | OpenCode | Agree | B2 mixed appends is the quality test. |
| opencode-m2 | OpenCode | Agree | Pin recall@10 vs A’s neighbors at `ef:100`; optional brute column, do not replace vs-A. |
| opencode-m1 | OpenCode | Agree | Assert puts actually insert (canary-hash early-return can no-op duplicates). |
| F-025-03 | Antigravity | Agree | If keep optimize: design note covers reverse-link / in-degree (shrink is outgoing-only). If kill: document periodic recreate. |
| F-025-04 | Antigravity | Agree | If keep: document exclusive write-txn duration. |
| F-025-06 / F-025-07 | Antigravity | Agree | If keep: tombstone compaction + degree vs neighbour count under multi-batch put. |
| F-025-02 | Antigravity | Decline | Keep handshake note; Ledgerful `HnswRefreshPlan` patch is OOS unless owner asks. |
| F-025-05 | Antigravity | Decline | 180s vs 35s / 2M lookups unmeasured; create also uses per-put `VectorCache`. Do not gate 025 on 023. |
| Nomic fixture | both | Decline | Seeded 021-shaped fixture is enough. |

## Last-PR Cursor comments

**N/A this track.** Empty GitHub PR scan (see 021).

## Tools (planning)

ledgerful + ai-brains used (vault empty). Live Ledgerful `HnswRefreshPlan` confirmed.

## Testing / Definition of Done

- [x] Quality vs time table in this track dir (`results.md`).
- [x] Kill or keep + proposed API if keep (**KILL**; no new sysop).
- [x] If keep: tests vs rebuild recall + existing HNSW suite. (N/A — killed. Smoke + 14k measurement tests landed.)

## Hard locks

No silent on-disk layout change. miette + `Result`.
