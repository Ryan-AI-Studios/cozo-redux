# Track 026 internal review — HNSW fast-build presets

**Harness:** Cursor Grok (internal, post-implement)
**Track:** `conductor/track026-hnsw-fast-build-presets`
**Branch:** `track/026-hnsw-fast-build-presets`
**Date:** 2026-09-02
**Scope:** spec + plan + results + fixture + tests + `runtime/mod.rs`. Read-only except this file.

## Summary

**Clean.** No critical / high / medium findings. Config measurement only: no `hnsw.rs` / `parse/sys.rs` edit, no Cozo parser default, no production `unwrap`/`expect`. Grid kept the `m16-ef20` kill probe, wrote a conversion row, and recorded DB bytes. **KILL** a faster Ledgerful preset is justified against spec thresholds (vs `(16,100)` ≥ 0.95 **and** vs brute ≥ 0.90).

## Findings above low

**None.**

## Hunt checklist

| Hunt | Result |
| :--- | :--- |
| `unwrap` / `expect` in production | **Pass.** New unwraps live only in `#[cfg(test)]` modules (`hnsw_fixture.rs`, `hnsw_fast_build_presets_test.rs`, existing test helpers in `hnsw_create_stats_test.rs`). `mod.rs` only adds those test modules. `hnsw.rs` / `parse/sys.rs` untouched. |
| Parser default changes (must be none) | **Pass.** `git diff origin/main` has no `parse/` files. Live `parse/sys.rs` still `bail!`s if `ef_construction == 0` or `m_neighbours == 0` (lines 613–618). Distance default remains L2; `extend_candidates` / `keep_pruned_connections` default **false**. |
| 14k smoke too slow | **Pass.** Always-on `hnsw_fast_build_presets_smoke` is N=**48**, dim=**8**, 3 tiny cells. 14k is `hnsw_fast_build_presets_14k` with `#[ignore]`. CI is `cargo nextest run --workspace` (ignored tests off). 021 smoke still 32×8. |
| Dropped `m16-ef20` kill probe | **Pass.** `GRID` lists `m:16, ef_construction:20, required: true` immediately after baseline. 14k test asserts that cell exists. `results.md` / `raw/grid.json`: vs-baseline **0.296**, vs-brute **0.270**. Optional skip is only `m8-ef20`. |
| Missing conversion row | **Pass.** `conversion_row` in `grid.json`: index `(16,100)`, query `ef:100`, vs brute **0.428**. Same number in `results.md`. Captured from the first (baseline) cell. |
| Keep/kill vs spec thresholds | **Pass.** Keep needs **both** vs `(16,100)` ≥ **0.95** and vs brute ≥ **0.90**. No cheaper cell meets vs-baseline 0.95 (best cheaper is `keep_pruned` **0.402**). No cell, including baseline, meets vs-brute 0.90. Kill clause (ef=20 unusable; 50/100 already right) fires. See matrix below. |
| Cozo defaults moved | **Pass.** No engine default change. HITL not requested. Fixture omits `keep_pruned` / `extend` when false, so create string stays `{ dim, dtype: F32, fields, distance: L2, m, ef_construction }`. |
| Placeholders | **Pass.** No TODO / FIXME / `unimplemented!` / placeholder SHA in track 026 files or the new tests. DoD checkboxes in `spec.md` are filled with measured numbers. |

## DoD matrix

| DoD | Status | Evidence |
| :--- | :--- | :--- |
| Preset table in track dir | **Pass** | `results.md` + `raw/grid.json` + `raw/cells.jsonl`. Numbers match (rounding only). |
| Recommended `ef_construction` / `m` for Ledgerful | **Pass** | Stay **`m: 16`, `ef_construction: 100`**. Do not enable `keep_pruned_connections`. Do not drop to `m: 8`. Ledgerful not edited. |
| Kill or keep recorded; HITL if parser defaults would change | **Pass** | **KILL** faster preset. Parser defaults **unchanged**; HITL not requested. |
| Fixture: dim 768, ~14k, SQLite, F32, L2, unit-normalized | **Pass** | Shared `hnsw_fixture.rs`: seed **21768**, N=**14000**, dim **768**. Query seed **21769**, 50 hold-outs, k=10, query `ef: 100`. Fresh sqlite per cell. |
| Grid: `ef` ∈ {20,40,50,100}, `m` ∈ {8,16}; keep `m=16 ef=20`; `keep_pruned` branch | **Pass** | Full m=16 sweep + `keep_pruned` `(16,40)` + `m=8 ef=100` (required). Extra `m8-ef50` / `m8-ef40` ran. Only optional `m8-ef20` skipped after 90 min (`skipped: ["m8-ef20"]`). `extend_candidates` stayed false. |
| Recall: both bands + one conversion row | **Pass** | Every cell has vs-baseline and vs-brute. Conversion **0.428**. Did not switch to brute-only. Did not drop vs-`(16,100)`. |
| Record DB file bytes | **Pass** | sqlite + `-wal` + `-shm` in table (baseline 270,921,728 B). |
| N not reduced | **Pass** | 14k kept; ~95 min Windows `--release` wall. |
| No engine speedup claim | **Pass** | Spec/plan/results: config measurement only. |

## Keep / kill vs spec thresholds

Keep if a cheaper cell meets **both**: recall@10 vs `(16,100)` at query `ef:100` ≥ **0.95**, **and** vs brute ≥ **0.90**.

Kill if recall at 20 is unusable and 50/100 is already the right default. Keep `ef=20` for `m=16` as the explicit kill probe.

Raw (`raw/grid.json`), N=14000 × 768, SQLite Windows `--release`, query `ef:100`, k=10, 50 queries:

| Cell | vs (16,100) | vs brute | vs-base ≥0.95? | vs-brute ≥0.90? | Both keep bands? |
| :--- | ---: | ---: | :--- | :--- | :--- |
| `m16-ef100` (baseline) | 1.000 | 0.428 | yes | **no** | **no** (not a cheaper preset) |
| `m16-ef20` (kill probe) | **0.296** | 0.270 | no | no | **no** |
| `m16-ef40` | 0.398 | 0.370 | no | no | **no** |
| `m16-ef50` | 0.396 | 0.360 | no | no | **no** |
| `m16-ef40 keep_pruned` | 0.402 | 0.372 | no | no | **no** |
| `m8-ef100` | 0.266 | 0.242 | no | no | **no** |
| `m8-ef50` | 0.236 | 0.184 | no | no | **no** |
| `m8-ef40` | 0.228 | 0.218 | no | no | **no** |
| `m8-ef20` | — | — | skipped (optional, budget) | — | — |

**KILL a faster Ledgerful preset: justified.**

- Kill probe `m16-ef20` is unusable (0.296 vs current index). Spec forbade dropping it; it ran.
- No cheaper cell is a drop-in for `(16,100)` (all vs-baseline ≤ 0.402, far below 0.95).
- `ef_construction: 40` / `50` at `m:16` overlap the current top-10 only ~40%.
- `keep_pruned` at `(16,40)` adds ~0.004 vs the no-pruned cell, costs ~2.2× that cell, and the DB is **larger** than baseline.
- `m: 8` is worse vs baseline than `m16-ef20` even at `ef_construction: 100`.
- vs-brute ≥ 0.90 is unmet even at production knobs on this **synthetic** unit-normalized 768-d corpus (conversion **0.428**). Spec required reporting that row, not switching to brute-only. The product-relevant gate (same neighbors as Ledgerful’s current index) independently kills every cheaper cell.

Recommend stay at **`m: 16`, `ef_construction: 100`**. This is a **config** kill (do not edit Ledgerful knobs). Cozo parser defaults must stay unset/required.

Conductor’s “**KEEP** (always)” on track 026 means the *measurement track* was worth running, not that a faster preset survived. Filled result is KILL the faster preset.

## Lows (do not block)

1. **Smoke does not lock the “vs brute = 1.0” wiring claim.** `results.md` says smoke recovered 1.0 vs brute; the test only asserts recall in `0.0..=1.0`. Comparators are still wired (conversion row + dual bands on every cell).
2. **`FIXTURE_N` is now shared** in `hnsw_fixture.rs`. Shrinking it would also shrink 021’s ignored 14k test. Documented in 021 `fixture.md`. N was not reduced.
3. **`m8-ef40` create wall (512 s) is slightly slower than `m8-ef50` (461 s).** Fresh sqlite per cell on Windows; does not change keep/kill.

## What looks solid

- Baseline is first in `GRID`, so vs-baseline = 1.0 by construction and the conversion row is exactly `(16,100)` vs brute.
- Optional 90 min skip cannot drop `m16-ef20` (`required: true`); 14k test asserts the probe ran.
- 021 helpers extracted without changing seed / normalize / `m:16` create string. 021 14k still does ef50 then drop + ef100.
- Hard lock create form preserved when extra flags are off.

## Research / tools notes

- ai-brains: used from `C:\dev\CozoDB-redux` (`preflight --summary`, `recall` / `sync query`). Vault already has the 026 KILL decision; matches this table.
- ledgerful: `doctor --json` `readyForPublish=true` (warns: graph content-stale, impact-stale, sig-pin, sig-version). `ledger status`: 1 pending TX `track026-hnsw-fast-build-presets` (FEATURE). `change-context --json` on the test/runtime paths: `status=ready`, risk=high from **including** `parse/sys.rs` in `--paths` plus historical runtime↔parser temporal coupling **79%**. Inspected live `parse/sys.rs`: **no diff**, defaults unchanged. `ledgerful search` failed (index lock / writer kill on `cozorocks` images); parser check was `Read` + `Grep` on `parse/sys.rs`. No `scan --impact` (read-only). Did not edit `.ledgerful` state.
- cargo 1.95.0. HEAD `2710891b` (021). 026 product files are uncommitted: `hnsw_fixture.rs` (new), `hnsw_fast_build_presets_test.rs` (new), `hnsw_create_stats_test.rs` (extract), `runtime/mod.rs` (`#[cfg(test)]` wiring only).

## Verdict

**Internal review clean.** No open finding above **low**. **KILL** the faster Ledgerful preset is justified. Handshake: keep **`m: 16`, `ef_construction: 100`**; no Cozo parser default change.
