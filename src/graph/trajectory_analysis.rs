use super::dijkstra::Dijkstra;
use super::path::Path;
use super::Graph;
use crate::lp::PreferenceEstimator;

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
}
