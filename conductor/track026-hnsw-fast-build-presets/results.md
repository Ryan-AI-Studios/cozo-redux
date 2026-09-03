# Track 026 results — Windows HNSW fast-build presets

Config measurement only. No engine speedup. No Cozo parser default change.

## Run

| Item | Value |
| :--- | :--- |
| Host | Windows (`x86_64-pc-windows-msvc`) |
| Engine | SQLite tempfile (fresh file **per cell**) |
| N × dim | **14000 × 768** F32, unit-normalized |
| Corpus seed | `StdRng` **21768** (same as 021) |
| Query seed | **21769**, 50 hold-out vectors, k=10, query `ef: 100` |
| Index | `snippet_embedding:snippet_idx`, L2, F32 |
| Profile | **`--release`** |
| Date | 2026-09-02 / 2026-09-03 (run crossed midnight) |
| Test wall | **5721 s** (~95 min), N **not** reduced |
| Skipped | `m=8, ef_construction=20` only (optional cell; elapsed 5721 s > 90 min budget). Required cells all ran. **`m=16, ef=20` ran** (kill probe). |

Raw: `conductor/track026-hnsw-fast-build-presets/raw/grid.json`, `cells.jsonl`.

Parser re-check at execute: `m` and `ef_construction` still **must be set** (`parse/sys.rs`); distance default L2; `extend_candidates` / `keep_pruned_connections` default **false**.

## Conversion row

Index `(m:16, ef_construction:100)` at query `ef:100` vs brute-force L2 top-10:

**recall@10 = 0.428**

Even Ledgerful’s production knobs do not meet the vs-brute ≥ 0.90 band on this **synthetic random** unit-normalized 768-d corpus. Distances concentrate in high-d; query `ef:100` is not enough to recover true top-10. The vs-`(16,100)` band is the product-relevant gate (would Ledgerful see the same neighbors?). Smoke (N=48, dim=8) recovered recall 1.0 vs brute, so the comparators are wired.

## Preset table

Keep band: vs `(16,100)` ≥ **0.95** **and** vs brute ≥ **0.90**. Speedup is vs this run’s `(16,100)` create wall (1,254,179 ms). DB bytes = sqlite + `-wal` + `-shm` after create (connection still open).

| m | ef_construction | keep_pruned | create ms | create min | vs (16,100) | vs brute | DB bytes | MiB | Keep? |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :--- |
| 16 | 100 | false | 1,254,179 | 20.90 | 1.000 | 0.428 | 270,921,728 | 258.4 | baseline (vs brute miss) |
| 16 | 20 | false | 206,364 | 3.44 | **0.296** | 0.270 | 142,450,688 | 135.9 | **KILL probe** |
| 16 | 40 | false | 503,582 | 8.39 | 0.398 | 0.370 | 202,240,000 | 192.9 | no |
| 16 | 50 | false | 699,864 | 11.66 | 0.396 | 0.360 | 227,340,288 | 216.8 | no |
| 16 | 40 | **true** | 1,120,663 | 18.68 | 0.402 | 0.372 | 277,495,808 | 264.6 | no (slower, bigger) |
| 8 | 100 | false | 874,976 | 14.58 | 0.266 | 0.242 | 172,654,592 | 164.7 | no |
| 8 | 50 | false | 461,144 | 7.69 | 0.236 | 0.184 | 172,421,120 | 164.4 | no |
| 8 | 40 | false | 512,203 | 8.54 | 0.228 | 0.218 | 171,667,456 | 163.7 | no |
| 8 | 20 | false | — | — | — | — | — | — | skipped (budget) |

`extend_candidates` stayed **false** (no extra cell).

## Recommendation (Ledgerful handshake)

**Do not change Ledgerful.** Keep the hardcoded create string at **`m: 16`, `ef_construction: 100`**. Do not enable `keep_pruned_connections`. Do not drop to `m: 8`.

- `ef_construction: 20` at `m: 16` is **unusable** (0.296 vs baseline). Ledgerful’s dim-3 integration-test value is not a production preset.
- `ef_construction: 40` and `50` at `m: 16` overlap the current index’s top-10 only ~40% — far below 0.95. They are **not** a drop-in for 100.
- `keep_pruned_connections: true` at `(16, 40)` adds ~0.004 recall vs the no-pruned cell and costs ~2.2× that cell’s create time (almost as slow as ef=100), with a **larger** DB than baseline.
- `m: 8` is worse vs baseline than `m: 16, ef=20` even at `ef_construction: 100`.

**No Cozo parser default change.** Keys remain required. HITL not requested.

## Keep / kill

**KILL** adopting a faster Ledgerful create preset from this grid.

Recall at `ef=20` is unusable, and `50`/`100` is already the right default for this corpus: no cheaper cell met **both** keep bands. vs-`(16,100)` ≥ 0.95 is only true for the baseline itself.

This is a **config** kill (do not edit Ledgerful knobs). The measurement table is the deliverable.
