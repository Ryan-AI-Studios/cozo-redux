# Track 026 plan

## Status

**Results filled.** Parser re-verified at execute: no default `m` / `ef_construction`. No `hnsw.rs` change. No Cozo defaults moved.

## Execute notes

Industry `ef_construction ≈ 64` for `m=16` was a sweep hypothesis, **not** adopted. Grid (N=**14000**, Windows `--release`): no cheaper cell met recall@10 vs `(16,100)` ≥ 0.95. Recommend Ledgerful stay at `m:16`, `ef_construction:100`. **KILL** faster preset. Optional `m8-ef20` skipped after 90 min; kill probe `m16-ef20` ran.

Nomic embeddings are not a required fixture. Shared helpers: `cozo-core/src/runtime/hnsw_fixture.rs`.

## Phases (map to DoD)

### 1. Sweep (DoD: table)

- Reuse 021 fixture **or** an identical/standalone generator (unit-normalized 768-d F32). Not blocked on 021 results.
- For each `(m, ef_construction)`: drop+create, record ms + DB file bytes, then query a fixed hold-out (e.g. 50 seeded query vectors, k=10, `ef: 100`).
- Keep `ef=20` at `m=16` as the kill probe. Note: a new node’s L0 **outgoing** set from one construction search is `≤ ef_construction`; later **reverse** edges can still raise degree. Do **not** drop `ef=20`.
- Add branch: `(m:16, ef_construction:40, keep_pruned_connections:true)` at minimum. `extend_candidates` stays default false unless a cheap extra cell is wanted.
- Baseline = `(16, 100)`. Recall vs that baseline **and** vs brute, with one conversion row (`ef:100` vs brute). Do not switch to brute-only. Do not forbid vs-`(16,100)`.

### 2. Recommend (DoD: handshake)

- Write `results.md` with recommendation for Ledgerful’s hardcoded create string.
- Do not edit Ledgerful unless asked.
- Do not change Cozo parser defaults without HITL.

### 3. Engine defaults (only if HITL)

- If ever: `parse/sys.rs` + changelog / BREAKING note.

## Files (expected)

- Scripts / notes under this track dir
- Engine only if defaults move

## Gate

If engine defaults change: fmt / clippy / tests + BREAKING or changelog. Otherwise markdown-only (plus ignored 14k test + smoke). Test helpers live in `hnsw_fixture.rs`.
