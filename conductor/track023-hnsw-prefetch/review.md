# Track 023 review — HNSW create prefetch

**Status:** KEEP. Create-scoped `VectorCache`. Publish via PR (number filled after `gh pr create`).

## DoD

| Item | Evidence |
| :--- | :--- |
| Create vs 021 | `results.md`: ef100 14k×768 SQLite Windows release **19.9 min → 12.6 min** (1.58×). `store_get_count=0`, `cache_instances=1` |
| Compact-single-threaded | `cargo check -p cozo --no-default-features --features compact-single-threaded` |
| KEEP recorded | `results.md` + conductor |
| No format break | RAM cache only; `stored.rs` incremental `hnsw_put` unchanged |
| Gates | fmt; clippy `-D warnings` `-p cozo --all-targets --all-features`; nextest `--lib --bins --workspace` (187 pass, 3 skipped) |
| RSS | Peak WS **124,383,232** bytes (~119 MiB); delta from pre-import **113 MiB**. Cap deferred as D-023-01 |

## Keep / kill

**KEEP** as default `::hnsw create`. HashMap, not columnar.

## Reviewer rounds

| Round | Result |
| :--- | :--- |
| Internal | clean; no findings above low (`internal-review.md`) |
| Cross-model | PASS WITH DEFERRED P3 (`review.codex.md`). D-023-01 RAM cap already in `docs/deferred.md`. |

## Publish

| Item | Value |
| :--- | :--- |
| Branch | `track/023-hnsw-prefetch` (deleted after squash) |
| PR | [#3](https://github.com/Ryan-AI-Studios/cozo-redux/pull/3) |
| SHA | `056d189a` |
