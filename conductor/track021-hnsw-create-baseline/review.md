# Track 021 review — HNSW create baseline

**Status:** measurement complete. Publish via PR (number filled after `gh pr create`).

## DoD

| Item | Evidence |
| :--- | :--- |
| Cost table | `results.md` + `raw/ef50.json` + `raw/ef100.json` |
| VectorCache lifetime | per-`hnsw_put`; `cache_instances == 14000` |
| Keep/kill 022–028 | filled in `spec.md`; conductor annotated |
| Fixture | SQLite, 14k×768 unit-norm, seed 21768, `m:16`, ef 50 and 100, Windows release |
| Engine counters | env-gated `COZO_HNSW_CREATE_STATS`; no tracing; no format change |
| Gates | fmt; clippy `-D warnings`; nextest `--lib --bins --workspace` (183 pass, 1 skipped); `cargo check -p cozo --no-default-features --features compact-single-threaded` |
| compact-single-threaded | pass (pre-existing unused-const warning without rayon) |

## Keep / kill (ef100, 14k×768, SQLite, Windows, release)

| Track | Call |
| :--- | :--- |
| 022 SIMD L2 | **KILL** (dist 10.3%) |
| 023 prefetch | **KEEP** (ensure_key 34.2%) |
| 024 put batching | **KILL** (put 0.94%) |
| 025 incremental | **KEEP** (~20 min create) |
| 026 knobs | **KEEP** (always) |
| 027 PQ | deferred (`train_pq` not run) |
| 028 GPU dist | **KILL** (dist 10.3%; batch ~20.8) |

Largest unowned bucket: graph/heaps **54.6%** → `docs/deferred.md` D-021-01.

## Reviewer rounds

| Round | Result |
| :--- | :--- |
| Internal | clean; no findings above low (`internal-review.md`) |
| Cross-model | PASS; three P3s (`review.codex.md`). Self-loop put routed through `hnsw_store_put`. Conductor kill notes applied. Process-global stats env deferred as D-021-02. |

## Publish

| Item | Value |
| :--- | :--- |
| Branch | `track/021-hnsw-create-baseline` (deleted after squash) |
| PR | [#1](https://github.com/Ryan-AI-Studios/cozo-redux/pull/1) |
| SHA | `2710891b` |
