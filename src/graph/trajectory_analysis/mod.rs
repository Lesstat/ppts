use super::dijkstra::Dijkstra;
use super::path::Path;
use super::Graph;
use crate::lp::PreferenceEstimator;


pub mod evaluations;

pub struct TrajectoryAnalysis<'a> {
    graph: &'a Graph,
}

pub struct SubPath {
    /// Index of the path where the SubPath starts
    pub start_index: u32,
    /// Index of the path where the SubPath ends
    pub end_index: u32,
}

impl<'a> TrajectoryAnalysis<'a> {
    pub fn new(graph: &'a Graph) -> TrajectoryAnalysis<'a> {
        TrajectoryAnalysis { graph }
    }

    pub fn find_non_optimal_segments(&self, path: &mut Path) -> Vec<SubPath> {
        if path.algo_split.is_none() {
            self.graph.find_preference(path);
        }
        let mut d = Dijkstra::new(&self.graph);

        // Ignore last cut as it is the last node of the path
        if let Some((_, cut_indices)) = path.algo_split.as_ref().and_then(|s| s.cuts.split_last()) {
            let mut res = Vec::new();
            for c in cut_indices {
                let mut dist = 1;
                loop {
                    let esti = PreferenceEstimator::new(&self.graph);
                    if esti
                        .calc_preference(&mut d, &path, c - dist, c + 1)
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
            res
        } else {
            Vec::new()
        }
    }
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
            .find_shortest_path(&mut d, 0, vec![0, 1], EQUAL_WEIGHTS)
            .unwrap();

        let ta = TrajectoryAnalysis::new(&linegraph);

        let non_opts = ta.find_non_optimal_segments(&mut path);

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
        //		     |	     1       |
        //		     +---------------+

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
            .find_shortest_path(&mut d, 0, vec![0, 2, 4], EQUAL_WEIGHTS)
            .unwrap();

        let ta = TrajectoryAnalysis::new(&graph);

        let non_opts = ta.find_non_optimal_segments(&mut path);

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
            .find_shortest_path(&mut d, 0, vec![0, 2, 5], EQUAL_WEIGHTS)
            .unwrap();

        let ta = TrajectoryAnalysis::new(&graph);

        let non_opts = ta.find_non_optimal_segments(&mut path);

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
            .find_shortest_path(&mut d, 0, vec![0, 2, 3, 6], EQUAL_WEIGHTS)
            .unwrap();

        let ta = TrajectoryAnalysis::new(&graph);

        let non_opts = ta.find_non_optimal_segments(&mut path);

        assert_eq!(2, non_opts.len());
        assert_eq!(1, non_opts[0].start_index);
        assert_eq!(3, non_opts[0].end_index);
        assert_eq!(2, non_opts[1].start_index);
        assert_eq!(4, non_opts[1].end_index);
    }

    /*
    Takes the optimal path cost vectors for each metric (e.g. alpha = (1,0,0)) and the cost vector of the actual path.
    Returns a scalar vector alpha that is an approximation of the underlying preferences of the actual path.
    If there are multiple paths with the same preference the input should be the sum of the paths' costs (both for the actual costs and the optimal costs).

    costs_per_metric: vector of scalar vectors, each should be the costs of the optimal path for a single metric
    real_costs: the cost vector of the path for which the preferences should be computed
    return value: approximation of the underlying preferences
     */
    fn get_linear_combination(costs_per_metric: &Vec<Vec<f64>>, real_costs: &Vec<f64>) -> Vec<f64> {
        let mut finished: bool = false;
        let dim: usize = real_costs.len();
        let mut alpha: Vec<f64> = vec![0.0; dim];
        let mut rest: Vec<f64> = real_costs.to_vec();
        let mut distance: f64 = get_length(&real_costs);
        let mut normalized_costs_per_metric: Vec<Vec<f64>> = Vec::new();
        for i in 0..dim {
            normalized_costs_per_metric.push(normalize_vec(&costs_per_metric[i]));
        }
        let mut count: usize = 0;
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
            if dist < 0.000001 {
                finished = true;
            } else {
                alpha[best_index] += best_scalar;
                distance = dist;
            }
            count += 1;
        }
        let mut sum_alpha: f64 = 0.0;
        for i in 0..dim {
            sum_alpha += alpha[i];
        }
        alpha.iter_mut().for_each(|x| *x /= sum_alpha);
        return alpha;
    }

    /*
    helper function for get_linear_combination
     */
    fn get_scalar_product(vec1: &Vec<f64>, vec2: &Vec<f64>) -> f64 {
        let mut res: f64 = 0.0;
        let dim: usize = vec1.len();
        for i in 0..dim {
            res += vec1[i] * vec2[i];
        }
        return res;
    }

    /*
    helper function for get_linear_combination
     */
    fn normalize_vec(vec: &Vec<f64>) -> Vec<f64> {
        let mut res: Vec<f64> = vec![0.0; vec.len()];
        let length: f64 = get_length(&vec);
        for i in 0..vec.len() {
            res[i] = vec[i] / length;
        }
        return res;
    }

    /*
    helper function for get_linear_combination
     */
    fn get_distance(vec1: &Vec<f64>, vec2: &Vec<f64>) -> f64 {
        let mut res: f64 = 0.0;
        for i in 0..vec1.len() {
            let dif: f64 = vec1[i] - vec2[i];
            res += dif * dif;
        }
        return res.sqrt();
    }

    /*
    helper function for get_linear_combination
     */
    fn get_length(vec: &Vec<f64>) -> f64 {
        let zeros = vec![0.0; vec.len()];
        return get_distance(&vec, &zeros);
    }
}
