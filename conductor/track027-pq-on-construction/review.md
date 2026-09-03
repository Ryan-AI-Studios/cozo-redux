# Track 027 review — PQ on construction

**Status:** KILL construction-PQ. L2 guard, centroid bound, re-rank. Publish via PR.

## DoD

| Item | Evidence |
| :--- | :--- |
| Gates (a)(b) | `results.md` + `raw/pq_14k.json` |
| KILL construction-PQ | dist 14.55% of create; no format change |
| L2 guard + centroids | tests `hnsw_train_pq_rejects_cosine`, `hnsw_train_pq_rejects_centroid_overflow` |
| Re-rank | `hnsw_pq_search_reranks_with_exact_l2` |
| Convert | `design.md` won’t-do; D-012-01 open |
| Gates | fmt; clippy `--all-targets --all-features -D warnings`; nextest; `cargo check -p cozo --no-default-features --features compact-single-threaded` |

## Reviewer rounds

| Round | Result |
| :--- | :--- |
| Internal | clean (`internal-review.md`) |
| Cross-model | first FAIL P3 RAM note. Documented + compact-single-threaded check. Fresh Codex: **PASS** (`review.codex.md`) |

## Publish

| Item | Value |
| :--- | :--- |
| Branch | `track/027-pq-on-construction` |
| PR | *(filled after open)* |
| SHA | *(filled after squash-merge)* |
