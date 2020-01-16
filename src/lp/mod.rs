use lp_modeler::dsl::{lp_sum, LpContinuous, LpExpression, LpObjective, LpOperations, LpProblem};
use lp_modeler::solvers::{GlpkSolver, SolverTrait};

use crate::graph::dijkstra::Dijkstra;
use crate::graph::path::Path;
use crate::graph::Graph;
use crate::helpers::{costs_by_alpha, Preference};
use crate::EDGE_COST_DIMENSION;

pub struct PreferenceEstimator<'a> {
    graph: &'a Graph,
    problem: LpProblem,
    variables: Vec<LpContinuous>,
    deltas: Vec<LpContinuous>,
    solver: GlpkSolver,
}
const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz";

impl<'a> PreferenceEstimator<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        let mut problem = LpProblem::new("Find Preference", LpObjective::Maximize);

        let mut variables = Vec::new();

        for tag in ALPHABET.chars().take(EDGE_COST_DIMENSION) {
            variables.push(LpContinuous::new(&tag.to_string()));
        }
        let deltas = Vec::new();

        // Constraints
        for var in &variables {
            problem += var.ge(0);
            if !var.name.contains("unit") {
                problem += var;
            }
        }
        problem += lp_sum(&variables).equal(1);

        PreferenceEstimator {
            graph,
            problem,
            variables,
            deltas,
            solver: GlpkSolver::new(),
        }
    }

    pub fn calc_preference(
        mut self,
        dijkstra: &mut Dijkstra,
        path: &Path,
        source_idx: u32,
        target_idx: u32,
    ) -> Option<Preference> {
        let costs = path.get_subpath_costs(self.graph, source_idx, target_idx);

        let mut prev_alphas: Vec<Preference> = Vec::new();
        let mut alpha = [1.0 / EDGE_COST_DIMENSION as f64; EDGE_COST_DIMENSION];
        prev_alphas.push(alpha);
        loop {
            // println!("find shortest path");
            let result = self
                .graph
                .find_shortest_path(
                    dijkstra,
                    0,
                    vec![path.nodes[source_idx], path.nodes[target_idx]],
                    alpha,
                )
                .unwrap();
            if &path.nodes[source_idx..=target_idx] == result.nodes.as_slice() {
                // Catch case paths are equal, but have slightly different costs (precision issue)
                return Some(alpha);
            } else if result.user_split.get_total_cost() >= costs_by_alpha(&costs, &alpha) {
                // println!(
                //     "Shouldn't happen: result: {:?}; user: {:?}",
                //     result.user_split.get_total_cost(),
                //     costs_by_alpha(costs, alpha)
                // );
                // dbg!(&costs, &result.total_dimension_costs, &alpha);
                let res = Some(alpha);
                return res;
            }
            let new_delta = LpContinuous::new(&format!("delta{}", self.deltas.len()));
            self.problem += new_delta.ge(0);
            self.problem += new_delta.clone();
            self.deltas.push(new_delta.clone());
            self.problem += (0..EDGE_COST_DIMENSION)
                .fold(LpExpression::ConsCont(new_delta), |acc, index| {
                    acc + LpExpression::ConsCont(self.variables[index].clone())
                        * ((costs[index] - result.total_dimension_costs[index]) as f32)
                })
                .le(0);

            match self.solve_lp() {
                Some(result) => {
                    if prev_alphas.iter().any(|a| a == &result) {
                        return None;
                    }
                    alpha = result;
                    prev_alphas.push(alpha);
                }
                None => return None,
            }
        }
    }

    fn solve_lp(&self) -> Option<Preference> {
        match self.solver.run(&self.problem) {
            Ok(solution) => {
                // println!("Solver Status: {:?}", status);
                let mut alpha = [0.0; EDGE_COST_DIMENSION];
                let mut all_zero = true;

                for (name, value) in solution.results.iter() {
                    if !name.contains("delta") {
                        if *value != 0.0 {
                            all_zero = false;
                        }
                        // The order of variables in the HashMap is not fixed
                        for (index, tag) in ALPHABET
                            .chars()
                            .take(EDGE_COST_DIMENSION)
                            .map(|c| c.to_string())
                            .enumerate()
                        {
                            if name == &tag {
                                alpha[index] = f64::from(*value);
                                break;
                            }
                        }
                    }
                }
                // println!("Alpha: {:?}", alpha);
                if all_zero {
                    return None;
                }
                Some(alpha)
            }
            Err(msg) => {
                println!("LpError: {}", msg);
                None
            }
        }
    }
}
