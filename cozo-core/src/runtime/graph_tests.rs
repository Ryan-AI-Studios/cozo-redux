/*
 * Copyright 2022, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::DbInstance;
use miette::Result;

#[test]
fn test_parallel_pagerank() -> Result<()> {
    let db = DbInstance::default();

    // Create a simple graph
    // A -> B
    // A -> C
    // B -> C
    // C -> A
    // D -> C
    db.run_default(":create edges {from, to}")
        .map_err(|e| miette::miette!(e.to_string()))?;
    db.run_default(
        "
        ?[from, to] <- [['A', 'B'], ['A', 'C'], ['B', 'C'], ['C', 'A'], ['D', 'C']]
        :put edges {from, to}
    ",
    )
    .map_err(|e| miette::miette!(e.to_string()))?;

    // Run PageRank
    let res = db
        .run_default(
            "
        ?[node, rank] <~ PageRank(*edges[_, _], iterations: 20)
    ",
        )
        .map_err(|e| miette::miette!(e.to_string()))?;

    let rows = res.into_json()["rows"]
        .as_array()
        .cloned()
        .ok_or_else(|| miette::miette!("No rows returned"))?;

    // We expect 4 nodes: A, B, C, D
    assert_eq!(rows.len(), 4);

    // Verify ranks are somewhat sensible (C should have the highest rank as it has most incoming edges)
    let mut ranks = std::collections::HashMap::new();
    for row in rows {
        let node = row[0]
            .as_str()
            .ok_or_else(|| miette::miette!("Node name is not a string"))?
            .to_string();
        let rank = row[1]
            .as_f64()
            .ok_or_else(|| miette::miette!("Rank is not a float"))?;
        ranks.insert(node, rank);
    }

    assert!(ranks.contains_key("A"));
    assert!(ranks.contains_key("B"));
    assert!(ranks.contains_key("C"));
    assert!(ranks.contains_key("D"));

    let rank_c = ranks
        .get("C")
        .ok_or_else(|| miette::miette!("Rank C missing"))?;
    let rank_a = ranks
        .get("A")
        .ok_or_else(|| miette::miette!("Rank A missing"))?;
    let rank_b = ranks
        .get("B")
        .ok_or_else(|| miette::miette!("Rank B missing"))?;
    let rank_d = ranks
        .get("D")
        .ok_or_else(|| miette::miette!("Rank D missing"))?;

    println!(
        "Ranks: A={}, B={}, C={}, D={}",
        rank_a, rank_b, rank_c, rank_d
    );

    // C has incoming edges from A, B, D.
    // A has incoming edge from C.
    // B has incoming edge from A.
    // D has no incoming edges.

    assert!(
        rank_c > rank_a,
        "Rank C ({}) should be > Rank A ({})",
        rank_c,
        rank_a
    );
    assert!(
        rank_c > rank_b,
        "Rank C ({}) should be > Rank B ({})",
        rank_c,
        rank_b
    );
    assert!(
        rank_c > rank_d,
        "Rank C ({}) should be > Rank D ({})",
        rank_c,
        rank_d
    );

    // D should have the lowest rank as it has no incoming edges (only damping factor contribution)
    assert!(rank_d < rank_a);
    assert!(rank_d < rank_b);

    // Exact value assertions (precision up to 1e-12)
    assert!(
        (rank_a - 0.372531533241272).abs() < 1e-12,
        "rank_a: {}",
        rank_a
    );
    assert!(
        (rank_b - 0.1958138346672058).abs() < 1e-12,
        "rank_b: {}",
        rank_b
    );
    assert!(
        (rank_c - 0.39415472745895386).abs() < 1e-12,
        "rank_c: {}",
        rank_c
    );
    assert!(
        (rank_d - 0.03749999403953552).abs() < 1e-12,
        "rank_d: {}",
        rank_d
    );

    Ok(())
}
