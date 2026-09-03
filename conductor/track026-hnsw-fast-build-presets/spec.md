# Track 026: Fast-build presets (`ef_construction` / `m`)

## Status

**Completed**

**KILL** a faster Ledgerful preset. Recommend stay at **`m: 16`, `ef_construction: 100`**. No Cozo parser default change.

## Objective

Measure Ledgerful-shaped create at several `ef_construction` / `m` values and publish a documented fast-build preset vs Ledgerful’s hardcoded `m:16`, `ef_construction:100`. Config-only for Ledgerful if keep; **no Cozo default change without HITL**.

Product north star: create time may drop with knobs Ledgerful already passes on the `::hnsw create` line, before any SIMD/I/O work.

## Plan-time snapshot (2026-09-02) — re-verify at execute

- Cozo parser: **no** default `m` / `ef_construction` (must be set). Distance default L2. **Re-verified at execute** (`parse/sys.rs` still bails if unset).
- Ledgerful production: `m:16`, `ef_construction:100`, query `ef: 100`. Integration test uses `ef_construction: 20` on a toy dim-3 index — **not** a production preset.
- Industry (plan-time, not a substitute for our table): `m=16` with `ef_construction` ≈ `2m`–`6m` is a common band; Ledgerful’s 100 is `6.25 × m`. Lower `ef_construction` usually cuts build time with recall risk.
- `extend_candidates` / `keep_pruned_connections`: **are** in live `HnswIndexConfig` (`parse/sys.rs` 94–95, parser 604–608, `relation.rs` manifest, `hnsw.rs` heuristic). Default **false**. In-scope as an optional grid branch, not OOS.

## Spike / kill

**Keep if:** a lower `ef_construction` (and/or `m: 8`) still meets recall for Ledgerful-shaped search. Keep **both** bands (recall@10 vs `ef_construction:100` index ≥ **0.95**, and vs brute force ≥ **0.90**) and require a **conversion row** (query `ef:100` vs brute, once). Do not switch to brute-only. Do not forbid vs-`(16,100)` recall.

**Kill if:** recall at 20 is unusable and 50/100 is already the right default for this corpus. Keep `ef=20` for `m=16` as the explicit kill probe — do **not** drop it from the grid.

Cozo engine defaults do **not** move without HITL even on keep. Keep may be “recommend Ledgerful `config.toml` / hardcoded create string”.

### Filled (2026-09-03, 14k×768 SQLite Windows release)

**KILL** lowering Ledgerful knobs. Table: `results.md`.

| Gate | Result |
| :--- | :--- |
| vs `(16,100)` ≥ 0.95 | Only the baseline itself. `m16-ef20` **0.296** (kill probe). `m16-ef40` 0.398, `m16-ef50` 0.396, `m8-ef100` 0.266. |
| vs brute ≥ 0.90 | **No cell**, including baseline. |
| Conversion row | `(16,100)` at query `ef:100` vs brute **0.428**. |
| `keep_pruned` `(16,40)` | vs-baseline 0.402; ~2.2× slower than `ef40` without it; DB larger than baseline. |
| Recommended preset | **`m: 16`, `ef_construction: 100`** (unchanged). Do not set `keep_pruned_connections`. |
| Cozo defaults | **Unchanged.** HITL not requested. Parser still requires `m` / `ef_construction`. |
| N | **14000** (not reduced). Optional `m8-ef20` skipped after 90 min budget; `m16-ef20` ran. |

## Requirements

1. Fixture: same shape as 021 (dim 768, ~14k, SQLite, F32, L2, **unit-normalized**). 026 may **share** the 021 generator **or duplicate** it. Not blocked waiting on 021 results. Standalone generator is OK. Nomic embeddings not required.
2. Grid: `ef_construction` ∈ {20, 40, 50, 100}; `m` ∈ {8, 16}. Full cartesian if cheap; if not, fix `m=16` and sweep `ef`, plus one `m=8, ef=100`. **Keep `ef=20` for `m=16` as the explicit kill probe** — do not drop it. Accurate note: a new node’s L0 **outgoing** set from one construction search is `≤ ef_construction`; later **reverse** edges can still raise degree. Add `keep_pruned_connections: true` as a grid branch (`m:16`, `ef:40`, `keep_pruned:true` at minimum). `extend_candidates` remains default false unless a cheap extra cell is wanted.
3. Table: create ms, recall@10, **record DB file bytes** in `results.md` (cheap; optional KV count still fine).
4. Recall: keep **both** bands (vs `(16,100)` at query `ef:100`, and vs brute force) but require a **conversion row** (query `ef:100` vs brute, once). Do not switch to brute-only. Do not forbid vs-`(16,100)` recall. Cosine rerank remains optional (index is L2; Ledgerful reranks `cos_dist`).

## Out of scope

SIMD (**022**). GPU. Changing HNSW math. Editing Ledgerful unless asked. Requiring Nomic embeddings. Dropping `ef=20` from the grid. Forbidding vs-`(16,100)` recall.

## Dependencies

None (parallel with 021). May share 021 generator or duplicate; not blocked on 021 results.

## §9 Deferred

None.

### Fold-in (2026-09-02)

| Id | Source | Disposition | Action |
| :--- | :--- | :--- | :--- |
| opencode-M1 / agy-F-026-01 | OpenCode + Antigravity | Agree | Spec was wrong: `extend_candidates` / `keep_pruned_connections` **are** in `HnswIndexConfig` (default false). Add `keep_pruned:true` grid branch (`m:16`, `ef:40` at minimum). |
| unit-normalized fixture | both | Agree | Same as 021; share generator or duplicate; not blocked on 021 results. |
| ef=20 kill probe | both | Agree | Keep `ef=20` for `m=16`. Note: L0 outgoing from one search ≤ `ef_construction`; reverse edges can still raise degree. |
| recall bands | both | Agree | Keep both vs-`(16,100)` and vs-brute; require one conversion row (`ef:100` vs brute). Do not switch to brute-only. |
| index size | both | Agree | Record DB file bytes in `results.md`. |
| Drop ef=20 | — | Decline | Keep as explicit kill probe. |
| Forbid vs-(16,100) | — | Decline | Keep vs-baseline recall. |
| Nomic embeddings | — | Decline | Seeded unit-normalized fixture is enough. |

## Last-PR Cursor comments

**N/A this track.** Empty GitHub PR scan (see 021).

## Tools (planning)

ledgerful + ai-brains used (vault empty). Live parser + Ledgerful create string confirmed.

## Testing / Definition of Done

- [x] Preset table in this track dir (`results.md`).
- [x] Recommended `ef_construction` / `m` for Ledgerful (handshake note): **keep `m:16`, `ef_construction:100`**.
- [x] Kill or keep recorded. HITL if Cozo parser defaults would change. **KILL** faster preset; **no** parser default change.

## Hard locks

Keep `::hnsw create { dim, dtype: F32, fields, distance: L2, m, ef_construction }` working. Do not invent parser defaults of 50 without HITL (live code currently **requires** the keys).
