# Track 029 results

## Workload

Ledgerful `query_candidates` remains a **single** `$query_vec` per call. That path does not batch parent tuples.

In-tree Cozo does: `?[...] := *queries{qid, qv}, ~docs:idx{id | query: qv, k, ef, bind_distance: dist}`. Owner call was **reframe as generic Cozo parent-join batches**, not kill-now.

## Keep / kill

**KEEP** the concurrent-read gate + chunked rayon parent loop.

| Backend | `is_concurrent_read_safe` | Parent loop |
| :--- | :--- | :--- |
| default | false | sequential |
| SQLite | **false** (explicit) | sequential |
| mem | true on `MemTx::Reader` only | parallel when rayon + chunk ≥ 8 |
| RocksDB (`cozorocks`) | true | parallel when rayon + chunk ≥ 8 |
| newrocks (`rocksdb::Transaction`) | false | sequential |
| fjall | true | parallel when rayon + chunk ≥ 8 |
| tikv / temp | default false | sequential |

No `unsafe`. `&SessionTx` is Send because `SessionTx: Sync`. SQLite stays sequential. `compact-single-threaded` has no rayon feature → sequential.

Immutable scripts open a read tx (`transact(false)`). `run_default` is mutable → mem Writer → sequential. HTTP/query immutable scripts hit the parallel path on mem/rocks/fjall.

Chunk size / threshold: **8**. `:limit` still pulls the iterator lazily; a chunk may compute extra k-NN internally, then the limiter stops.

The spec’s ≥2× keep bar is **not a merge gate** for this search-side KEEP. Owner reframed 029 as generic Cozo parent-join correctness (concurrent-safe backends, SQLite sequential). No wall-clock table vs sequential is recorded.

## D-009-01

Close after this KEEP lands on main.
