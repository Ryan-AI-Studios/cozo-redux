# Track 026 review — HNSW fast-build presets

**Status:** measurement complete. **KILL** a faster Ledgerful preset. Stay at `m: 16`, `ef_construction: 100`.

## DoD

| Item | Evidence |
| :--- | :--- |
| Preset table | `results.md` + `raw/grid.json` |
| Ledgerful handshake | keep hardcoded `m:16`, `ef_construction:100` |
| Kill/keep | **KILL** faster preset; no Cozo parser default change |
| Fixture | shared `hnsw_fixture.rs`; 14k×768 unit-norm; `m16-ef20` ran |
| Conversion row | query `ef:100` vs brute **0.428** on `(16,100)` |
| Gates | fmt; clippy; nextest `--lib` (14k ignored) |

## Reviewer rounds

| Round | Result |
| :--- | :--- |
| Internal | clean; no findings above low (`internal-review.md`) |
| Cross-model | PASS (`review.codex.md`). Easy P3s: 14k asserts keep_pruned/N/conversion; smoke vs-brute ≥ 0.99. Applied. |

## Publish

| Item | Value |
| :--- | :--- |
| Branch | `track/026-hnsw-fast-build-presets` (deleted after squash) |
| PR | [#2](https://github.com/Ryan-AI-Studios/cozo-redux/pull/2) |
| SHA | `0447d02a` |
