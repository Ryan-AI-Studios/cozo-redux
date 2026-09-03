# Fold-in note — Track 029

**Date:** 2026-09-02
**Sources:** `opencode-review.md`, `agy-review.md`
**Status after fold-in:** Ready — not started (not Completed)

Verified live: `StoreTx: Sync` (`storage/mod.rs` ~55), no `Send` supertrait; `SessionTx` holds `Box<dyn StoreTx<'a> + 'a>` (`transact.rs` ~24–30) — type-legal Sync; no `Clone`; `is_concurrent_read_safe` absent; `SqliteTx` `unsafe impl Sync` + `stmts[GET_QUERY]: Mutex`; `HnswSearchRA::iter` is a lazy `map_ok` over parent (`ra.rs` ~1171–1206).

## Dispositions

| Id | Source | Sev | Disposition | Action |
| :--- | :--- | :--- | :--- | :--- |
| OpenCode M1 “SessionTx is not Sync” | OpenCode | M | Decline (typing) | Spec corrected: `SessionTx` **is** Sync. |
| opencode-M1 hatch | OpenCode | M | Agree — partial | If sharing requires `unsafe`, kill or HITL. |
| agy-T029-M02 | Agy | M | Agree — fold | Struck “cloning read-only handles”. |
| SQLite concurrent-safe | Agy B01 | B | Decline | Default stays false. Do not keep a SQLite-true-only design. |
| agy-T029-M01 | Agy | M | Agree — fold (if keep) | Lazy iter; bounded chunks; honor `:limit`. |
| agy-T029-m01 | Agy | m | Agree — fold (if keep) | `map_init` stacks. |
| agy-T029-B02 | Agy | B | Agree — fold (if keep) | Backend matrix; false backends sequential without error. |
| Close D-009-01 now | Agy / old Phase 1 | — | Decline | Row stays open. Notes updated in `docs/deferred.md`. |
| Invented 2–5× worse | Agy B01 | — | Decline | Not a measured fact. |

Not folded (outside owner fold list): T029-M03 shared `VectorCache`, T029-O01 `cfg` pairing (already implied by compact-single-threaded).

## Escalate to owner

Phase 1 still finds Ledgerful single-`$query_vec` and no in-tree batched `HnswSearchRA` caller. **Do not close D-009-01 in this fold-in.** Choose:

1. **Kill-now** after the Phase 1 workload note (leave sequential; close the row then), or
2. **Reframe** as generic Cozo parent-join batches (`?[...] := *batch_queries[q], ~idx{... | query: q}`) on concurrent-safe backends (mem/rocks/fjall), with SQLite remaining sequential/`false`.
