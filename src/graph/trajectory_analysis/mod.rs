use super::dijkstra::Dijkstra;
use super::path::{Path, PathSplit};
use super::Graph;
use crate::helpers::{Costs, MyVec, Preference};
use crate::lp::{LpProcess, PreferenceEstimator};
use crate::MyResult;
use crate::EDGE_COST_DIMENSION;

pub mod evaluations;

pub struct TrajectoryAnalysis<'a, 'b> {
    graph: &'a Graph,
    dijkstra: &'b mut Dijkstra<'a>,
    lp: &'b mut LpProcess,
}

pub struct SubPath {
    /// Index of the path where the SubPath starts
    pub start_index: u32,
    /// Index of the path where the SubPath ends
    pub end_index: u32,
}

impl<'a, 'b> TrajectoryAnalysis<'a, 'b> {
    pub fn new(
        graph: &'a Graph,
        dijkstra: &'b mut Dijkstra<'a>,
        lp: &'b mut LpProcess,
    ) -> TrajectoryAnalysis<'a, 'b> {
        TrajectoryAnalysis {
            graph,
            dijkstra,
            lp,
        }
    }

    pub fn find_preference(&mut self, path: &mut Path) -> MyResult<()> {
        let path_length = path.nodes.len() as u32;
        let mut cuts = MyVec::new();
        let mut alphas = MyVec::new();
        let mut start = 0u32;

        while start < path_length - 1 {
            let mut low = start;
            let mut high = path_length;
            let mut best_pref = None;
            let mut best_cut = 0;
            loop {
                let m = (low + high) / 2;
                if start == m {
                    return Ok(());
                }
                let mut estimator = PreferenceEstimator::new(self.graph, self.lp);
                let pref = estimator.calc_preference(self.dijkstra, &path, start, m)?;
                if pref.is_some() {
                    low = m + 1;
                    best_pref = pref;
                    best_cut = m;
                } else {
                    high = m;
                }
                if low >= high {
                    alphas.push(best_pref.unwrap());
                    cuts.push(best_cut);
                    break;
                }
            }
            start = best_cut;
        }
        let dimension_costs = MyVec::new();
        let costs_by_alpha = MyVec::new();
        path.algo_split = Some(PathSplit {
            cuts,
            alphas,
            dimension_costs,
            costs_by_alpha,
        });

        Ok(())
    }
    pub fn find_non_optimal_segments(&mut self, path: &mut Path) -> MyResult<Vec<SubPath>> {
        if path.algo_split.is_none() {
            self.find_preference(path)?;
        }

        // Ignore last cut as it is the last node of the path
        if let Some((_, cut_indices)) = path.algo_split.as_ref().and_then(|s| s.cuts.split_last()) {
            let mut res = Vec::new();
            for c in cut_indices {
                let mut dist = 1;
                loop {
                    let mut esti = PreferenceEstimator::new(&self.graph, self.lp);
                    if esti
                        .calc_preference(self.dijkstra, &path, c - dist, c + 1)?
                        .is_none()
                    {
                        let subpath = SubPath {
                            start_index: c - dist,
                            end_index: c + 1,
                        };
                        res.push(subpath);
                        break;
                    }
                    dist += 1;
                }
            }
            Ok(res)
        } else {
            Ok(Vec::new())
        }
    }

   /* pub fn find_all_non_optimal_segments(&mut self, path: &mut Path) -> MyResult<Vec<SubPath>> {
        let mut subpaths = Vec::new();
        let mut start: i32 = 0;
        let lenPath: u32 = path.nodes.len() as u32;
        let mut stop: i32 = lenPath as i32;
        let mut esti = PreferenceEstimator::new(&self.graph, self.lp);
        while start != stop {
            let mut step = (stop - start + 1) / 2;
            while step > 0 {
                if esti
                    .calc_preference(self.dijkstra, &path, start as u32, stop as u32)?
                    .is_none()
                {
                    start += step;
                    if start >= stop {
                        start = stop - 1;
                    }
                } else {
                    if start == 0 {
                        return Ok(subpaths);
                    }
                    start -= step;
                    if start < 0 {
                        start = 0;
                    }
                }
                if step == 1 {
                    step = 0;
                } else {
                    step = (step + 1) / 2;
                }
            }
            if esti
                .calc_preference(self.dijkstra, &path, start as u32, stop as u32)?
                .is_some()
            {
                if start == 0 {
                    return Ok(subpaths);
                }
                start -= 1;
            }
            let subpath = SubPath {
                start_index: start as u32,
                end_index: stop as u32,
            };
            subpaths.push(subpath);
        }

        // Ignore last cut as it is the last node of the path
        if let Some((_, cut_indices)) = path.algo_split.as_ref().and_then(|s| s.cuts.split_last()) {
            let mut res = Vec::new();
            for c in cut_indices {
                let mut dist = 1;
                loop {
                    let mut esti = PreferenceEstimator::new(&self.graph, self.lp);
                    if esti
                        .calc_preference(self.dijkstra, &path, c - dist, c + 1)?
                        .is_none()
                    {
                        let subpath = SubPath {
                            start_index: c - dist,
                            end_index: c + 1,
                        };
                        res.push(subpath);
                        break;
                    }
                    dist += 1;
                }
            }
            Ok(res)
        } else {
            Ok(Vec::new())
        }
    }*/

    pub fn get_single_preference_decomposition(
        &mut self,
        contraint_paths: &Vec<Path>,
        path: &Path,
    ) -> MyResult<SinglePreferenceDecomposition> {
        let path_length = path.nodes.len() as u32;
        let mut cuts = MyVec::new();
        let mut start = 0u32;
        let mut best_pref = None;
        let mut best_subpath = path.get_subpath(self.graph, start, start);
        let mut paths = contraint_paths.clone();
        let mut estimator = PreferenceEstimator::new(self.graph, self.lp);
        while start < path_length - 1 {
            let mut low = start;
            let mut high = path_length;
            let mut best_cut = 0;
            loop {
                let m = (low + high) / 2;
                if start == m {
                    let res = SinglePreferenceDecomposition{ cuts, preference : [-1.0; EDGE_COST_DIMENSION]};
                    return Ok(res);
                }
                let subpath = path.get_subpath(self.graph, start, m);
                paths.push(subpath.clone());
                let pref = estimator.calc_preference_for_multiple_paths(self.dijkstra, &paths)?;
                paths.pop();
                if pref.is_some() {
                    low = m + 1;
                    best_pref = pref;
                    best_cut = m;
                    best_subpath = subpath;
                } else {
                    high = m;
                }
                if low >= high {
                    cuts.push(best_cut);
                    paths.push(best_subpath.clone());
                    break;
                }
            }
            start = best_cut;
        }
        let res = SinglePreferenceDecomposition{ cuts, preference : best_pref.unwrap()};
        Ok(res)
    }
}

pub struct SinglePreferenceDecomposition {
    pub preference: Preference,
    pub cuts: MyVec<u32>,
}

/// Takes the optimal path cost vectors for each metric (e.g. alpha =
/// (1,0,0)) and the cost vector of the actual path.
///
/// Returns a scalar vector alpha that is an approximation of the
/// underlying preferences of the actual path.  If there are multiple
/// paths with the same preference the input should be the sum of the
/// paths' costs (both for the actual costs and the optimal costs).
///
/// costs_per_metric: vector of scalar vectors, each should be the
/// costs of the optimal path for a single metric real_costs: the cost
/// vector of the path for which the preferences should be computed
/// return value: approximation of the underlying preferences
pub fn get_linear_combination(costs_per_metric: &[Costs], real_costs: &[f64]) -> Preference {
    let mut finished: bool = false;
    let dim: usize = real_costs.len();
    let mut alpha = [0.0; EDGE_COST_DIMENSION];
    let mut rest: Vec<f64> = real_costs.to_vec();
    //let mut distance: f64 = get_length(&real_costs); distance was set but never used
    let mut normalized_costs_per_metric: Vec<Vec<f64>> = Vec::new();
    for metric_cost in costs_per_metric {
        normalized_costs_per_metric.push(normalize_vec(metric_cost));
    }
    // let mut count: usize = 0; count was set but never used
    while !finished {
        let mut best_scalar: f64 = 0.0;
        let mut best_index: usize = 0;
        for i in 0..dim {
            let mut scalar: f64 = get_scalar_product(&normalized_costs_per_metric[i], &rest);
            if scalar + alpha[i] < 0.0 {
                scalar = -alpha[i];
            }
            if scalar.abs() > best_scalar.abs() {
                best_scalar = scalar;
                best_index = i;
            }
        }
        let best_metric: &Vec<f64> = &normalized_costs_per_metric[best_index];
        let mut step: Vec<f64> = vec![0.0; dim];
        for i in 0..dim {
            step[i] = best_scalar * best_metric[i];
            rest[i] -= step[i];
        }
        let dist: f64 = get_length(&step);
        if dist < 0.000_001 {
            finished = true;
        } else {
            alpha[best_index] += best_scalar;
            //distance = dist;
        }
        // count += 1;
    }
    let sum_alpha: f64 = alpha.iter().sum();

    alpha.iter_mut().for_each(|x| *x /= sum_alpha);
    alpha
}

/*
helper function for get_linear_combination
 */
pub fn get_scalar_product(vec1: &[f64], vec2: &[f64]) -> f64 {
    let mut res: f64 = 0.0;
    let dim: usize = vec1.len();
    for i in 0..dim {
        res += vec1[i] * vec2[i];
    }
    res
}

/*
helper function for get_linear_combination
 */
fn normalize_vec(vec: &[f64]) -> Vec<f64> {
    let mut res: Vec<f64> = vec![0.0; vec.len()];
    let length: f64 = get_length(&vec);
    for i in 0..vec.len() {
        res[i] = vec[i] / length;
    }
    res
}

/*
helper function for get_linear_combination
 */
fn get_distance(vec1: &[f64], vec2: &[f64]) -> f64 {
    let mut res: f64 = 0.0;
    for i in 0..vec1.len() {
        let dif: f64 = vec1[i] - vec2[i];
        res += dif * dif;
    }
    res.sqrt()
}

/*
helper function for get_linear_combination
 */
pub fn get_length(vec: &[f64]) -> f64 {
    let zeros = vec![0.0; vec.len()];
    get_distance(&vec, &zeros)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::*;
    use crate::helpers::EQUAL_WEIGHTS;
    use crate::EDGE_COST_DIMENSION;

    #[test]
    fn test_no_non_optimal_subpath() {
        let linegraph = Graph::new(
            vec![Node::new(0, 0), Node::new(1, 0)],
            vec![Edge::new(0, 0, 1, [1.0; EDGE_COST_DIMENSION], None)],
        );

        let mut d = Dijkstra::new(&linegraph);

        let mut path = linegraph
            .find_shortest_path(&mut d, 0, &[0, 1], EQUAL_WEIGHTS)
            .unwrap();

        let mut lp = LpProcess::new().unwrap();

        let mut ta = TrajectoryAnalysis::new(&linegraph, &mut d, &mut lp);

        let non_opts = ta.find_non_optimal_segments(&mut path).unwrap();

        assert!(non_opts.is_empty());
    }

    #[test]
    fn test_single_non_optimal_subpath() {
        // Ascii art of the graph
        // s,t,x = vertices
        // +,-,| = part of an edge
        //	       	 1     	  1  	 1       1
        //	     s-------x-------x-------x------t
        //		     |		     |
        //		     |	     1   |
        //		     +-----------+

        let one_cost = [1.0; EDGE_COST_DIMENSION];

        let graph = Graph::new(
            vec![
                Node::new(0, 0), //s
                Node::new(1, 0),
                Node::new(2, 0),
                Node::new(3, 0),
                Node::new(4, 0), // t
            ],
            vec![
                Edge::new(0, 0, 1, one_cost, None),
                Edge::new(1, 1, 2, one_cost, None),
                Edge::new(2, 2, 3, one_cost, None),
                Edge::new(3, 3, 4, one_cost, None),
                Edge::new(4, 1, 3, one_cost, None), // lower edge that skips one node
            ],
        );

        let mut d = Dijkstra::new(&graph);
        let mut path = graph
            .find_shortest_path(&mut d, 0, &[0, 2, 4], EQUAL_WEIGHTS)
            .unwrap();
        let mut lp = LpProcess::new().unwrap();
        let mut ta = TrajectoryAnalysis::new(&graph, &mut d, &mut lp);

        let non_opts = ta.find_non_optimal_segments(&mut path).unwrap();

        assert_eq!(1, non_opts.len());
        assert_eq!(1, non_opts[0].start_index);
        assert_eq!(3, non_opts[0].end_index);
    }

    #[test]
    fn test_long_non_optimal_subpath() {
        // Ascii art of the graph
        // s,t,x = vertices
        // +,-,| = part of an edge
        //	       	 1     1   1	 1       1
        //	     s-------x---x---x-------x------t
        //		     |		     |
        //		     |	     1       |
        //		     +---------------+

        let one_cost = [1.0; EDGE_COST_DIMENSION];

        let graph = Graph::new(
            vec![
                Node::new(0, 0), //s
                Node::new(1, 0),
                Node::new(2, 0),
                Node::new(3, 0),
                Node::new(4, 0),
                Node::new(5, 0), // t
            ],
            vec![
                Edge::new(0, 0, 1, one_cost, None),
                Edge::new(1, 1, 2, one_cost, None),
                Edge::new(2, 2, 3, one_cost, None),
                Edge::new(3, 3, 4, one_cost, None),
                Edge::new(4, 4, 5, one_cost, None),
                Edge::new(5, 1, 4, one_cost, None), // lower edge that skips two nodes
            ],
        );

        let mut d = Dijkstra::new(&graph);
        let mut path = graph
            .find_shortest_path(&mut d, 0, &[0, 2, 5], EQUAL_WEIGHTS)
            .unwrap();
        let mut lp = LpProcess::new().unwrap();
        let mut ta = TrajectoryAnalysis::new(&graph, &mut d, &mut lp);

        let non_opts = ta.find_non_optimal_segments(&mut path).unwrap();

        assert_eq!(1, non_opts.len());
        assert_eq!(1, non_opts[0].start_index);
        assert_eq!(4, non_opts[0].end_index);
    }

    #[test]
    fn test_finding_overlapping_non_optimal_subpaths() {
        // Ascii art of the graph
        // s,t,x = vertices
        // +,-,| = part of an edge
        //                                   1
        //			     +---------------+
        //			     |		     |
        //	       	 1       1   |	 1       1   |
        //	     s-------x-------x-------x-------x-----t
        //		     |		     |
        //		     |	     1       |
        //		     +---------------+

        let one_cost = [1.0; EDGE_COST_DIMENSION];

        let graph = Graph::new(
            vec![
                Node::new(0, 0), //s
                Node::new(1, 0),
                Node::new(2, 0),
                Node::new(3, 0),
                Node::new(4, 0),
                Node::new(5, 0),
                Node::new(6, 0), // t
            ],
            vec![
                Edge::new(0, 0, 1, one_cost, None),
                Edge::new(1, 1, 2, one_cost, None),
                Edge::new(2, 2, 3, one_cost, None),
                Edge::new(3, 3, 4, one_cost, None),
                Edge::new(4, 4, 5, one_cost, None),
                Edge::new(5, 4, 6, one_cost, None),
                Edge::new(6, 1, 3, one_cost, None), // lower edge that skips one node
                Edge::new(7, 2, 4, one_cost, None), // upper edge that skips one node
            ],
        );

        let mut d = Dijkstra::new(&graph);
        let mut path = graph
            .find_shortest_path(&mut d, 0, &[0, 2, 3, 6], EQUAL_WEIGHTS)
            .unwrap();
        let mut lp = LpProcess::new().unwrap();
        let mut ta = TrajectoryAnalysis::new(&graph, &mut d, &mut lp);

        let non_opts = ta.find_non_optimal_segments(&mut path).unwrap();

        assert_eq!(2, non_opts.len());
        assert_eq!(1, non_opts[0].start_index);
        assert_eq!(3, non_opts[0].end_index);
        assert_eq!(2, non_opts[1].start_index);
        assert_eq!(4, non_opts[1].end_index);
    }
}
