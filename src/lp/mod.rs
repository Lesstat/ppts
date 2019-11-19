use lp_modeler::operations::LpOperations;
use lp_modeler::problem::{LpObjective, LpProblem};
use lp_modeler::solvers::{GlpkSolver, SolverTrait};
use lp_modeler::variables::{lp_sum, LpContinuous, LpExpression};

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

        // Variables
        let mut variables = Vec::new();

        for tag in ALPHABET.chars().take(EDGE_COST_DIMENSION) {
            variables.push(LpContinuous::new(&tag.to_string()));
        }
        let deltas = Vec::new();

        // Constraints
        for var in &variables {
            problem += var.ge(0);
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

    /*
        pub fn calc_preference(
        &mut self,
        driven_routes: &[Path],
        alpha: Preference,
    ) -> Option<Preference> {
        let current_feasible = self.check_feasibility(driven_routes, alpha);
        if current_feasible {
        return Some(alpha);
    }
        while let Some(alpha) = self.solve_lp() {
        let feasible = self.check_feasibility(driven_routes, alpha);
        if feasible {
        return Some(alpha);
    }
    }
        None
    }
         */

    pub fn calc_preference(
        &mut self,
        path: &Path,
        source_idx: usize,
        target_idx: usize,
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
                    0,
                    vec![path.nodes[source_idx], path.nodes[target_idx]],
                    alpha,
                )
                .unwrap();
            if &path.nodes[source_idx..=target_idx] == result.nodes.as_slice() {
                // Catch case paths are equal, but have slightly different costs (precision issue)
                return Some(alpha);
            } else if result.user_split.get_total_cost() >= costs_by_alpha(costs, alpha) {
                // println!(
                //     "Shouldn't happen: result: {:?}; user: {:?}",
                //     result.user_split.get_total_cost(),
                //     costs_by_alpha(costs, alpha)
                // );
                // dbg!(&costs, &result.total_dimension_costs, &alpha);
                return Some(alpha);
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
                    if result == alpha {
                        return Some(alpha);
                    }
                    alpha = result;
                    if prev_alphas.iter().any(|a| a == &alpha) {
                        // println!("looping alphas");
                        return Some(alpha);
                    }
                    prev_alphas.push(alpha);
                    // println!("alpha: {:?}", alpha);
                }
                None => return None,
            }
        }
    }

    /*
        fn check_feasibility(&mut self, driven_routes: &[Path], alpha: Preference) -> bool {
        let mut all_explained = true;
        for route in driven_routes {
        let source = route.nodes[0];
        let target = route.nodes[route.nodes.len() - 1];
        let result = self
        .graph
        .find_shortest_path(vec![source, target], alpha)
        .unwrap();
        if route.nodes == result.nodes {
        println!("Paths are equal, proceed with next route");
    } else if costs_by_alpha(route.costs, alpha) > result.total_cost {
        all_explained = false;
        println!(
        "Not explained, {} > {}",
        costs_by_alpha(route.costs, alpha),
        result.total_cost
    );
        let new_delta = LpContinuous::new(&format!("delta{}", self.deltas.len()));
        self.problem += new_delta.ge(0);
        self.problem += new_delta.clone();
        self.deltas.push(new_delta.clone());
        self.problem += (0..EDGE_COST_DIMENSION)
        .fold(LpExpression::ConsCont(new_delta), |acc, index| {
        acc + LpExpression::ConsCont(self.variables[index].clone())
         * ((route.costs[index] - result.costs[index]) as f32)
    })
        .le(0);
    }
    }
        all_explained
    }
         */

    fn solve_lp(&self) -> Option<Preference> {
        /*
        self.problem
        .write_lp("lp_formulation")
        .expect("Could not write LP to file");
         */
        match self.solver.run(&self.problem) {
            Ok((_status, var_values)) => {
                // println!("Solver Status: {:?}", status);
                let mut alpha = [0.0; EDGE_COST_DIMENSION];
                let mut all_zero = true;
                for (name, value) in var_values.iter() {
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

#[cfg(test)]
mod tests {
    use super::*;
}
