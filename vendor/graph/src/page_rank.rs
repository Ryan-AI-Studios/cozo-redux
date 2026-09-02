use crate::prelude::*;

use log::info;
use rayon::prelude::*;

use std::time::Instant;

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct PageRankConfig {
    /// The maximum number of page rank iterations.
    #[cfg_attr(feature = "clap", clap(long, default_value_t = PageRankConfig::DEFAULT_MAX_ITERATIONS))]
    pub max_iterations: usize,

    /// If the sum of page rank deltas per iteration is
    /// below the tolerance value, the computation stop.
    #[cfg_attr(feature = "clap", clap(long, default_value_t = PageRankConfig::DEFAULT_TOLERANCE))]
    pub tolerance: f64,

    /// Imagining a random surfer clicking links, the
    /// damping factor defines the probability if the
    /// surfer will continue at any step.
    #[cfg_attr(feature = "clap", clap(long, default_value_t = PageRankConfig::DEFAULT_DAMPING_FACTOR))]
    pub damping_factor: f32,
}

impl Default for PageRankConfig {
    fn default() -> Self {
        Self {
            max_iterations: Self::DEFAULT_MAX_ITERATIONS,
            tolerance: Self::DEFAULT_TOLERANCE,
            damping_factor: Self::DEFAULT_DAMPING_FACTOR,
        }
    }
}

impl PageRankConfig {
    pub const DEFAULT_MAX_ITERATIONS: usize = 20;
    pub const DEFAULT_TOLERANCE: f64 = 1E-4;
    pub const DEFAULT_DAMPING_FACTOR: f32 = 0.85;

    pub fn new(max_iterations: usize, tolerance: f64, damping_factor: f32) -> Self {
        Self {
            max_iterations,
            tolerance,
            damping_factor,
        }
    }
}

pub fn page_rank<NI, G>(graph: &G, config: PageRankConfig) -> (Vec<f32>, usize, f64)
where
    NI: Idx,
    G: Graph<NI> + DirectedDegrees<NI> + DirectedNeighbors<NI> + Sync,
{
    let PageRankConfig {
        max_iterations,
        tolerance,
        damping_factor,
    } = config;

    let node_count = graph.node_count().index();
    let init_score = 1_f32 / node_count as f32;
    let base_score = (1.0_f32 - damping_factor) / node_count as f32;

    let mut out_scores = Vec::with_capacity(node_count);

    (0..node_count)
        .into_par_iter()
        .map(NI::new)
        .map(|node| init_score / graph.out_degree(node).index() as f32)
        .collect_into_vec(&mut out_scores);

    let mut scores = vec![init_score; node_count];
    let mut next_scores = vec![0.0_f32; node_count];
    let mut next_out_scores = vec![0.0_f32; node_count];

    let mut iteration = 0;

    loop {
        let start = Instant::now();
        let error = page_rank_iteration(
            graph,
            base_score,
            damping_factor,
            &out_scores,
            &scores,
            &mut next_out_scores,
            &mut next_scores,
        );

        std::mem::swap(&mut scores, &mut next_scores);
        std::mem::swap(&mut out_scores, &mut next_out_scores);

        info!(
            "Finished iteration {} with an error of {:.6} in {:?}",
            iteration,
            error,
            start.elapsed()
        );

        iteration += 1;

        if error < tolerance || iteration == max_iterations {
            return (scores, iteration, error);
        }
    }
}

fn page_rank_iteration<NI, G>(
    graph: &G,
    base_score: f32,
    damping_factor: f32,
    out_scores: &[f32],
    scores: &[f32],
    next_out_scores: &mut [f32],
    next_scores: &mut [f32],
) -> f64
where
    NI: Idx,
    G: Graph<NI> + DirectedDegrees<NI> + DirectedNeighbors<NI> + Sync,
{
    next_scores
        .par_iter_mut()
        .zip(next_out_scores.par_iter_mut())
        .enumerate()
        .map(|(u_idx, (next_score, next_out_score))| {
            let u = NI::new(u_idx);
            let incoming_total = graph
                .in_neighbors(u)
                .map(|v| out_scores[v.index()])
                .sum::<f32>();

            let old_score = scores[u_idx];
            let new_score = base_score + damping_factor * incoming_total;
            *next_score = new_score;
            *next_out_score = new_score / graph.out_degree(u).index() as f32;

            f64::abs((new_score - old_score) as f64)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::{CsrLayout, GraphBuilder};

    #[test]
    fn test_pr_two_components() {
        let gdl = "(a)-->()-->()<--(a),(b)-->()-->()<--(b)";

        let graph: DirectedCsrGraph<usize> = GraphBuilder::new()
            .csr_layout(CsrLayout::Sorted)
            .gdl_str::<usize, _>(gdl)
            .build()
            .unwrap();

        let (scores, _, _) = page_rank(&graph, PageRankConfig::default());

        let expected: Vec<f32> = vec![
            0.024999997,
            0.035624996,
            0.06590624,
            0.024999997,
            0.035624996,
            0.06590624,
        ];

        assert_eq!(scores, expected);
    }
}
