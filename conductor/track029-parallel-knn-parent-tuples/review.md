# Track 029 review — parallel parent-tuple k-NN

**Status:** KEEP concurrent-read gate + chunked search. Publish via PR.

## DoD

| Item | Evidence |
| :--- | :--- |
| Workload | `results.md` — Ledgerful single `$query_vec`; Cozo `*queries, ~idx` is the batch |
| KEEP, no unsafe | `StoreTx::is_concurrent_read_safe`; `&SessionTx` Send via Sync |
| SQLite false | explicit override + `sqlite_is_not_concurrent_read_safe` |
| Lazy chunks + `:limit` | chunk 8; `mem_parent_knn_limit_stops_at_five` |
| `map_init` stacks | rayon path; `mem_parent_knn_filter_even_ids` |
| Sequential vs parallel | mem Reader vs Writer golden |
| compact-single-threaded | `cargo check -p cozo --no-default-features --features compact-single-threaded` |
| Gates | fmt; clippy `--all-targets --all-features -D warnings`; nextest `--lib --bins --workspace` |

## Reviewer rounds

| Round | Result |
| :--- | :--- |
| Internal | clean (`internal-review.md`) |
| Cross-model | **PASS** (`review.codex.md`). P3 sqlite flag assertion fixed. Remaining P3s: fjall/rocks count-only smoke; ≥2× not a merge gate. |

## Publish

| Item | Value |
| :--- | :--- |
| Branch | `track/029-parallel-knn-parent-tuples` |
| PR | *(filled after open)* |
| SHA | *(filled after squash-merge)* |
