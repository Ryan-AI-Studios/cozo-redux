# Track Completion Audit — track026-hnsw-fast-build-presets

**Reviewer:** Codex completion audit (`review.codex.md`)
**Date:** 2026-09-02
**Branch:** `track/026-hnsw-fast-build-presets` at HEAD `2710891b` (same as `origin/main`); implementation is uncommitted working tree + track artifacts
**Track dir:** `C:\dev\CozoDB-redux\conductor\track026-hnsw-fast-build-presets`

## Verdict: PASS

## Scope Reviewed

- Spec + plan: `conductor/track026-hnsw-fast-build-presets/spec.md`, `plan.md` (read in full)
- Results / fixture: `results.md`, `fixture.md`, `raw/grid.json`, `raw/cells.jsonl`
- Working tree vs `origin/main`:
  - Product: `cozo-core/src/runtime/mod.rs` (test module wiring only)
  - New test-only: `cozo-core/src/runtime/hnsw_fixture.rs`, `hnsw_fast_build_presets_test.rs`
  - 021 reuse: `cozo-core/src/runtime/hnsw_create_stats_test.rs` now calls shared fixture
  - Docs: `conductor.md` 026 → In progress; 021 fixture path note for shared `FIXTURE_N`
- Unchanged vs `origin/main` (verified empty diff): `cozo-core/src/parse/sys.rs`, `cozo-core/src/runtime/hnsw.rs`, `BREAKING.md`
- Live parser: `parse/sys.rs` still bails if `m` / `ef_construction` unset; `keep_pruned_connections` / `extend_candidates` default false
- `docs/deferred.md` (no 026-owned open rows)
- Not in this repo: Ledgerful `config.toml` / create string (spec: do not edit unless asked)

## Requirement and DoD Matrix

| Requirement | Met/Partial/Unmet/Not verifiable | Evidence | Tests | Gap |
| :--- | :--- | :--- | :--- | :--- |
| Fixture: dim 768, N~14k, SQLite, F32, L2, unit-normalized; share or duplicate 021 generator | **Met** | `hnsw_fixture.rs`: `FIXTURE_SEED=21768`, `FIXTURE_DIM=768`, `FIXTURE_N=14000`, `unit_normalized_vecs`, SQLite tempfile. 021 test imports the same helpers. `grid.json` `"n": 14000`, `"dim": 768`, `"seed": 21768` | smoke uses tiny N/dim; 14k ignored test calls `FIXTURE_N` / `FIXTURE_DIM` | None |
| Grid: `ef_construction` ∈ {20,40,50,100}, `m` ∈ {8,16}; keep `m=16,ef=20` kill probe; `keep_pruned` branch `(16,40)` min | **Met** | `GRID` in `hnsw_fast_build_presets_test.rs` 41–96: kill probe `required: true`; keep_pruned cell `required: true`. `grid.json` has `m16-ef20` and `m16-ef40-keep_pruned`. Optional `m8-ef20` skipped after budget (`"skipped": ["m8-ef20"]`). Spec allows non-full cartesian if not cheap | 14k test asserts kill probe + baseline present | Optional `m8-ef20` not run — allowed |
| Table: create ms, recall@10, DB file bytes in `results.md` | **Met** | `results.md` preset table matches `grid.json` (rounded). `sqlite_db_bytes` sums sqlite + `-wal` + `-shm` | smoke asserts `create_ms > 0` and `db_bytes > 0` | None |
| Recall: both bands (vs `(16,100)` and vs brute) + one conversion row (query `ef:100` vs brute). Do not switch to brute-only | **Met** | Every cell has `recall_at_k_vs_baseline` and `recall_at_k_vs_brute`. Conversion: `grid.json` `"conversion_row": { "index": "m:16,ef_construction:100", "query_ef": 100, "recall_at_k_vs_brute": 0.428 }` | smoke records both bands; 14k test does not assert conversion numeric | None |
| DoD: preset table in track dir | **Met** | `results.md` | n/a | None |
| DoD: recommended Ledgerful `m:16`, `ef_construction:100` | **Met** | `results.md` Recommendation; spec Status | n/a | None |
| DoD: kill/keep recorded; HITL if Cozo defaults would change | **Met** | **KILL** faster Ledgerful preset. HITL not requested. Parser unchanged | n/a | None |
| Hard lock: `::hnsw create { dim, dtype: F32, fields, distance: L2, m, ef_construction }` still works; no invented parser defaults | **Met** | `hnsw_create_cfg` emits that shape. `git diff origin/main -- parse/sys.rs` empty. Live: `ef_construction must be set` / `m_neighbours must be set` (`parse/sys.rs` 613–617) | smoke + 021 smoke create indexes | None |
| Test helpers test-only; no engine / parser default change | **Met** | `mod.rs`: `#[cfg(test)] mod hnsw_fixture` / `hnsw_fast_build_presets_test`. No `hnsw.rs` diff | n/a | None |
| N=14000 (not reduced) | **Met** | `grid.json` `"n": 14000`; `FIXTURE_N = 14_000`; results.md “N **not** reduced” | 14k test passes `FIXTURE_N` into `run_grid` | None |
| `keep_pruned` cell actually ran (flag wired) | **Met** | `grid.json` cell `m16-ef40-keep_pruned`: `keep_pruned_connections: true`, create 1,120,663 ms vs 503,582 ms for `m16-ef40`, DB 277,495,808 vs 202,240,000. Wiring: parser → `relation.rs` 1166 → `hnsw.rs` 756–766 | smoke includes `keep_pruned: true` cell; 14k test does not assert this cell | None |
| No Cozo default change | **Met** | Empty diff on `parse/sys.rs` / `hnsw.rs` | n/a | None |

Focus checks (kill probe, conversion, both bands, no default change, test-only helpers, keep_pruned, N=14000): all **Met**.

## Findings

No P0–P2.

### [P3] Ignored 14k test does not lock keep_pruned / conversion / N

Confidence: High
Requirement: Grid keep_pruned branch; conversion row; N=14000
Location: `cozo-core\src\runtime\hnsw_fast_build_presets_test.rs:371-382`
Problem: `hnsw_fast_build_presets_14k` only asserts that `m=16,ef=20` and `m=16,ef=100` appear in `cells`. It does not assert the keep_pruned cell, `conversion_row`, or `"n": 14000`.
Evidence: Assertions at 371–382; artifacts in `raw/grid.json` already contain those fields from the executed run.
Failure scenario: A future edit could skip keep_pruned or shrink N while the ignored test still passes, if `raw/grid.json` were not re-checked.
Correction: Assert keep_pruned cell present, `conversion_row.recall_at_k_vs_brute` is a number, and `out["n"] == FIXTURE_N`.
Verification: Re-run is not required for this track’s filled table; add asserts before landing the test helper if desired.
Deferrable: No (easy). Does not block this measurement DoD — raw grid already has the cells.

### [P3] Smoke recall asserts range only

Confidence: High
Requirement: Both recall comparators wired
Location: `cozo-core\src\runtime\hnsw_fast_build_presets_test.rs:339-352`
Problem: Smoke requires recall in `[0.0, 1.0]`, not a high vs-brute floor.
Evidence: This review re-ran smoke: all three cells `recall_at_k_vs_brute: 1.0` (N=48, dim=8, k=5). `results.md` documents that. The assert would still pass at 0.0.
Failure scenario: A broken brute or search comparator could still green the always-on test.
Correction: Assert smoke conversion / vs-brute at 1.0 (or ≥ 0.99) on this tiny corpus.
Verification: `cargo test -p cozo --lib hnsw_fast_build_presets_smoke` (observed pass with 1.0).
Deferrable: No (easy). Not a 14k DoD miss.

## Completeness Sweep

Searched new/changed HNSW test files for `TODO` / `FIXME` / `XXX` / `HACK` / `unimplemented!` / `todo!` / `panic!("TODO")`: none.

- No production stubs. `hnsw_fixture.rs` `.unwrap()` is test-only (`#[cfg(test)]`).
- `#[ignore]` on `hnsw_fast_build_presets_14k` is intentional (plan: ignored 14k + smoke in `nextest --lib`).
- Default features: `compact` → `minimal` → `storage-sqlite`, so `#[cfg(feature = "storage-sqlite")]` smoke is in the lib test gate.
- Budget skip applies only to `required: false` m=8 lower-ef cells; kill probe cannot skip. Observed skip: `m8-ef20` only.
- `raw/cells.jsonl` has eight 14k cells plus three later smoke lines (k=5). Authoritative 14k dump is `grid.json` (no smoke cells). Not a DoD miss.
- No SIMD/GPU/feature-gated dead code claimed as shipped.
- `docs/deferred.md`: no 026 row to close. D-021-01/02 unrelated.

## Wiring and Regression Review

```
::hnsw create { dim, dtype: F32, fields, distance: L2, m, ef_construction [, keep_pruned_connections] }
  -> parse/sys.rs HnswIndexConfig (m / ef required; keep_pruned default false)
  -> db.rs SysOp::CreateVectorIndex
  -> relation.rs create_hnsw_index_body copies keep_pruned_connections into HnswIndexManifest (line 1166)
  -> hnsw_put / select_neighbours heuristic (hnsw.rs 756–766)
  -> ~snippet_embedding:snippet_idx search (query ef:100, k:10)
  -> recall vs baseline neighbor IDs and vs brute L2² top-10
```

- Engine math unchanged. Config-only measurement as specified.
- `keep_pruned` is not a no-op on the 14k cell: ~2.2× create vs `m16-ef40`, DB larger than baseline (277,495,808 > 270,921,728).
- `compact-single-threaded`: no engine edit; plan does not require that gate for markdown + test helpers.
- Determinism: seeded `StdRng` corpus 21768 / queries 21769; brute ranking uses L2² (same order as L2).
- Storage: SQLite tempfile per cell (spec). RocksDB/fjall not in this grid.
- Windows paths: `COZO_HNSW_FAST_BUILD_OUT` directory join; sqlite `-wal`/`-shm` suffix via `OsString::push`.
- 021 smoke still passes after fixture extraction (observed this review).
- Ledgerful change-context flagged `HnswCreateCfg` / `callback` / `db` / `hnsw` as “public”; those are `pub(super)` / existing `pub(crate)` modules. `HnswCreateCfg` is not a shipped API. Risk “high” is from a new **test** env var `COZO_HNSW_FAST_BUILD_OUT`, same pattern as 021’s stats out-dir.

## Verification Evidence

**Observed now**

- `cargo test -p cozo --lib hnsw_fast_build_presets_smoke -- --nocapture` — pass; vs-brute 1.0 on all three smoke cells including keep_pruned.
- `cargo test -p cozo --lib hnsw_create_stats_smoke -- --nocapture` — pass after shared fixture.
- `git diff origin/main -- cozo-core/src/parse/sys.rs cozo-core/src/runtime/hnsw.rs BREAKING.md` — empty.
- `raw/grid.json`: `n: 14000`, conversion 0.428, kill probe `m16-ef20` vs-baseline 0.296, keep_pruned cell present, skipped `m8-ef20`.
- `results.md` table numbers match `grid.json` (thousand-separated / 3-decimal rounding).
- `ledgerful doctor --json`: `readyForPublish: true`, block 0, warn 4 (index stale, sig pin/version — pre-existing).
- `ledgerful change-context --json` on the four runtime paths: `status: ready`, `ledger.pending=1` (`219d87b0` entity `track026-hnsw-fast-build-presets`).
- `ledgerful ledger status --compact`: 1 pending, 0 unaudited drift.
- `ai-brains preflight --summary` + recall: pinned KILL stay at `m:16` / `ef_construction:100`; conversion 0.428. Matches artifacts.
- `ai-brains sync query`: FTS hit the same decision; index merge hit Windows `Access is denied` on tantivy (search degraded, recall still worked).

**Reported by orchestrator (not re-run here)**

- `ledgerful verify --scope fast` pass.
- 14k ignored test ran with N=14000 (~5721 s wall). Consistent with summed create_ms in `grid.json` (~5633 s create + queries).

**Recommended (not required to keep this PASS)**

- Optional: tighten 14k/smoke asserts (P3s above).
- Publish still needs ledger commit of pending TX + conductor status flip; out of this review’s write set.

**Not verifiable here**

- Re-execution of the 14k `--release --ignored` grid (hours; artifacts used instead).
- Ledgerful production create-string file (different repo; spec forbids editing unless asked).
- `ledgerful verify --scope fast` this session (not run; implementer-reported only).

## Deferred Candidates

None that belong in `docs/deferred.md`. The two P3s are easy test-lock tightenings, not difficult leftovers. vs-brute ≥ 0.90 failing on this synthetic 768-d corpus (even at production knobs, conversion 0.428) is the **kill result**, not a deferral. Spec kept the vs-`(16,100)` band as product-relevant; this work did not switch to brute-only.

## Completion Decision

**PASS.** Track 026 is a config measurement. The Windows 14k×768 SQLite `--release` grid ran required cells: kill probe `m16-ef20` (vs-baseline **0.296**), keep_pruned `(16,40)`, m=16 ef sweep, `m8-ef100` (plus optional m8-ef50/40). Conversion row is **0.428**. No cheaper cell met vs-`(16,100)` ≥ 0.95 or vs-brute ≥ 0.90. Recommendation **stay `m: 16`, `ef_construction: 100`**. Cozo parser defaults were not changed. Helpers are `#[cfg(test)]`. N stayed **14000**.

Conductor `In progress` until publish is what the spec says. P3 test-lock items do not reopen the keep/kill.
