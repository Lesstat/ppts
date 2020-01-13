use super::path::Path;
use super::Graph;

pub struct TrajectoryAnalysis<'a> {
    graph: &'a Graph,
}

pub struct SubPath<'a> {
    edges: &'a [u32],
}

impl<'a> TrajectoryAnalysis<'a> {
    pub fn new(graph: &'a Graph) -> TrajectoryAnalysis<'a> {
        TrajectoryAnalysis { graph }
    }

    pub fn find_non_optimal_segments<'b>(&self, path: &'b mut Path) -> Vec<SubPath<'b>> {
        if path.algo_split.is_none() {
            self.graph.find_preference(path);
        }

        // Ignore last cut as it is the last node of the path
        if let Some((_, cut_indices)) = path.algo_split.as_ref().and_then(|s| s.cuts.split_last()) {
            let mut res = Vec::new();
            for c in cut_indices {
                let subpath = SubPath {
                    edges: &path.edges[(c - 1)..=*c],
                };
                res.push(subpath);
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
    use crate::graph::dijkstra::*;
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
        assert_eq!(&[1, 2], non_opts[0].edges);
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
        assert_eq!(&[1, 2, 3], non_opts[0].edges);
    }
}
