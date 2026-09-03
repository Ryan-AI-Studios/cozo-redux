# Track 025 internal review

Kill is justified: 14k B2 recall@10 vs A is 0.54 (band 0.90) and B2 wall 16.0 min > A 11.9 min. Appends are one `$data` `:put` per 500-row batch (`put_mode=one_script_per_batch`). No optimize sysop. `stored.rs` unchanged. Puts asserted via distinct `fr_id`.

Codex P2 (singleton writes vs claimed 8×500 batches) validated and fixed by remasure.

No findings above low.
