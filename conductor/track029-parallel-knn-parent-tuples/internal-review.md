# Track 029 internal review

KEEP concurrent-read gate + chunked parent-tuple k-NN. Ledgerful is still single `$query_vec`; the workload is generic Cozo `*queries, ~idx{ query: qv }`.

| DoD | Evidence |
| :--- | :--- |
| Workload note | `results.md` — Ledgerful single-query; Cozo parent-join real |
| Keep, no unsafe | `StoreTx::is_concurrent_read_safe`; `&SessionTx` is Send via Sync; no `unsafe` |
| SQLite false | explicit `false` in `sqlite.rs`; test `sqlite_parent_knn_works_without_concurrent_reads` |
| Lazy chunks + `:limit` | chunk 8; `QueryLimiter` pulls iterator; `mem_parent_knn_limit_stops_at_five` |
| `map_init` stacks | rayon path in `HnswSearchRA::iter`; filter test `mem_parent_knn_filter_even_ids` |
| Sequential vs parallel | `mem_parent_knn_parallel_matches_sequential_on_same_db` (Reader parallel vs Writer sequential) |
| compact-single-threaded | `cargo check -p cozo --no-default-features --features compact-single-threaded` |
| Gates | fmt `--check`; clippy `--all-targets --all-features -D warnings`; nextest `--lib --bins --workspace` 198 passed |

No findings above low.
