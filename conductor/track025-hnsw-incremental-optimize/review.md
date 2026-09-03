# Track 025 review — incremental HNSW vs rebuild

**Status:** KILL skip-drop. Publish via PR.

## DoD

| Item | Evidence |
| :--- | :--- |
| Quality vs time | `results.md` + `raw/incremental_14k.json` |
| KILL + no new API | recall 0.54 < 0.90; B2 16.0 min > A 11.9 min |
| Puts insert | node-count assert per batch |
| B2 cadence | one `$data` `:put` per 500-row batch (`put_mode=one_script_per_batch`) |
| Gates | fmt; clippy `--all-targets --all-features -D warnings`; nextest `--lib --bins --workspace` |
| compact-single-threaded | no engine change |

## Reviewer rounds

| Round | Result |
| :--- | :--- |
| Internal | clean; kill is evidence-based (`internal-review.md`) |
| Cross-model | first FAIL on P2 (singleton writes). Fixed + remasured. Fresh Codex: **PASS** (`review.codex.md`) |

## Publish

| Item | Value |
| :--- | :--- |
| Branch | `track/025-hnsw-incremental-optimize` (deleted after squash) |
| PR | [#4](https://github.com/Ryan-AI-Studios/cozo-redux/pull/4) |
| SHA | `0899e084` |
