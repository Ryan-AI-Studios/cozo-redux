# Track 021 fixture — Ledgerful-shaped HNSW create

Repeatable SQLite rebuild used for the Windows cost table in `results.md`.

## Shape

| Knob | Value |
| :--- | :--- |
| Engine | SQLite tempfile (`DbInstance::new("sqlite", path, "")`) |
| Relation | `snippet_embedding {id: Int => embedding: <F32; 768>}` |
| Index | `snippet_embedding:snippet_idx` |
| N | **14000** (10k–20k is in spec range) |
| Dim / dtype | 768 / F32 |
| Distance | L2 |
| `m` | 16 |
| `ef_construction` | **50** then drop + create **100** |
| Vectors | Seeded **unit-normalized** F32 |
| RNG | `rand::rngs::StdRng` seed **21768** (`021768` = track 021 + dim 768) |
| PQ | none (`train_pq` not run) |

Generation: each row draws 768 uniform `f32` values in `[-1, 1]`, then L2-normalizes (matches Ledgerful normalize-before-put). Import is `DbInstance::import_relations` with `NamedRows` and `DataValue::Vec(Box::new(Vector::F32(Array1::from(v))))`.

Shared helpers (021 + 026): `cozo-core/src/runtime/hnsw_fixture.rs` (`FIXTURE_SEED` / `FIXTURE_DIM` / `FIXTURE_N`, generator, sqlite tempfile, `::hnsw create` / drop). If a **release** 14k run OOMs or exceeds ~45 minutes, set `FIXTURE_N` in that file (not in `hnsw_create_stats_test.rs`).

## Commands (PowerShell)

From `C:\dev\CozoDB-redux`. Do not overlap cargo jobs. Never use `&&`.

Always-on smoke (fast; stays in `nextest --lib`):

```powershell
cargo test -p cozo --lib hnsw_create_stats_smoke -- --nocapture
```

14k baseline (ignored; many minutes). **Use `--release` on Windows** — debug was still inside the first create after ~29 min with no snapshot.

```powershell
$env:COZO_HNSW_CREATE_STATS='1'
$env:COZO_HNSW_CREATE_STATS_OUT='C:\dev\CozoDB-redux\conductor\track021-hnsw-create-baseline\raw'
cargo test -p cozo --lib hnsw_create_baseline_14k --release -- --ignored --nocapture
```

Debug (no `--release`) is the literal unit-test command; do not use it for the cost table. If a **release** 14k run OOMs or exceeds ~45 minutes, set `FIXTURE_N` in `cozo-core/src/runtime/hnsw_fixture.rs` to `10_000` (still in spec range) and record that in `results.md`. The 2026-09-02 Windows run kept N=14000 in release (~31 min test wall).

JSON snapshots:

- stderr: one line per `::hnsw create` (also `ef50` / `ef100` prefixes from the test)
- if `COZO_HNSW_CREATE_STATS_OUT` is a directory: `ef50.json` and `ef100.json`

Env gate: `COZO_HNSW_CREATE_STATS=1` or `true` (case-insensitive). Tests call `hnsw_create_stats::reset()` **after** setting the env var. Default off; no `tracing` crate.

## Compact / single-threaded

After engine edits:

```powershell
cargo check -p cozo --no-default-features --features compact-single-threaded
```

(Package name is `cozo`; the crate lives in `cozo-core/`.)
