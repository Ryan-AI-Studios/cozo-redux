# Track 026 fixture — fast-build preset grid

Shares the 021 generator. Helpers live in `cozo-core/src/runtime/hnsw_fixture.rs` so 026 does not duplicate the 021 module.

## Shape (same as 021)

| Knob | Value |
| :--- | :--- |
| Engine | SQLite tempfile (`DbInstance::new("sqlite", path, "")`) |
| Relation | `snippet_embedding {id: Int => embedding: <F32; 768>}` |
| Index | `snippet_embedding:snippet_idx` |
| N | **14000** |
| Dim / dtype | 768 / F32 |
| Distance | L2 |
| Vectors | Seeded **unit-normalized** F32 |
| Corpus RNG | `StdRng` seed **21768** |
| Query RNG | seed **21769** (hold-out, 50 vectors) |
| Query | k=10, `ef: 100` |
| PQ | none |

Each grid cell uses a **fresh** SQLite tempfile (import + create) so DB file bytes are comparable. `extend_candidates` stays false. Extra cell: `m:16`, `ef_construction:40`, `keep_pruned_connections: true`.

## Commands (PowerShell)

From `C:\dev\CozoDB-redux`. Do not overlap cargo jobs. Never use `&&`.

Always-on smoke (tiny N/dim; stays in `nextest --lib`):

```powershell
cargo test -p cozo --lib hnsw_fast_build_presets_smoke -- --nocapture
```

14k grid (ignored; many minutes). **Use `--release` on Windows.**

```powershell
$env:COZO_HNSW_FAST_BUILD_OUT='C:\dev\CozoDB-redux\conductor\track026-hnsw-fast-build-presets\raw'
cargo test -p cozo --lib hnsw_fast_build_presets_14k --release -- --ignored --nocapture
```

If elapsed exceeds ~90 min after the required cells (`m=16` sweep, `m=16 ef=40 keep_pruned`, `m=8 ef=100`), optional `m=8` lower-ef cells are skipped. `m=16 ef=20` is never skipped.

The 2026-09-03 Windows `--release` run kept **N=14000**. Optional `m8-ef20` was skipped (5721 s > 90 min). `m8-ef50` and `m8-ef40` still ran. Kill probe `m16-ef20` ran.

JSON:

- stderr: one `cell {…}` line per create
- if `COZO_HNSW_FAST_BUILD_OUT` is a directory: `cells.jsonl` and `grid.json`
