use crate::graph::dijkstra::Dijkstra;
use crate::graph::path::Path;
use crate::graph::Graph;
use crate::helpers::{costs_by_alpha, Preference};
use crate::EDGE_COST_DIMENSION;

pub struct PreferenceEstimator<'a, 'b> {
    graph: &'a Graph,
    lp: &'b mut LpProcess,
}

impl<'a, 'b> PreferenceEstimator<'a, 'b> {
    pub fn new(graph: &'a Graph, lp: &'b mut LpProcess) -> Self {
        lp.reset().expect("Could not reset lp");
        PreferenceEstimator { graph, lp }
    }

    pub fn calc_preference(
        &mut self,
        dijkstra: &mut Dijkstra,
        path: &Path,
        source_idx: u32,
        target_idx: u32,
    ) -> MyResult<Option<Preference>> {
        self.lp.reset().expect("LP Process could not be reset");
        let costs = path.get_subpath_costs(self.graph, source_idx, target_idx);

        let mut prev_alphas: Vec<Preference> = Vec::new();
        let mut alpha = [1.0 / EDGE_COST_DIMENSION as f64; EDGE_COST_DIMENSION];
        prev_alphas.push(alpha);
        loop {
            let result = self
                .graph
                .find_shortest_path(
                    dijkstra,
                    0,
                    &[path.nodes[source_idx], path.nodes[target_idx]],
                    alpha,
                )
                .unwrap();
            if &path.nodes[source_idx..=target_idx] == result.nodes.as_slice() {
                // Catch case paths are equal, but have slightly different costs (precision issue)
                return Ok(Some(alpha));
            } else if result.user_split.get_total_cost() >= costs_by_alpha(&costs, &alpha) {
                // println!(
                //     "Shouldn't happen: result: {:?}; user: {:?}",
                //     result.user_split.get_total_cost(),
                //     costs_by_alpha(costs, alpha)
                // );
                // dbg!(&costs, &result.total_dimension_costs, &alpha);
                let res = Some(alpha);
                return Ok(res);
            }
            let mut cost_dif: Costs = [0.0; EDGE_COST_DIMENSION];

            cost_dif
                .iter_mut()
                .zip(costs.iter().zip(result.total_dimension_costs.iter()))
                .for_each(|(c, (p, r))| *c = r - p);

            self.lp.add_constraint(&cost_dif)?;
            match self.lp.solve()? {
                Some((pref, delta)) => {
                    if delta < 0.0 {
                        return Ok(None);
                    }
                    if prev_alphas.iter().any(|a| a == &pref) {
                        return Ok(None);
                    }
                    alpha = pref;
                    prev_alphas.push(alpha);
                }
                None => return Ok(None),
            }
        }
    }

    pub fn calc_representative_preference(
        &mut self,
        dijkstra: &mut Dijkstra,
        path: &Path,
    ) -> MyResult<Preference> {
        self.lp.reset().expect("LP Process could not be reset");
        let costs = path.total_dimension_costs;

        let accuracy = 0.0001;

        let mut best_dif = f64::MAX;
        let mut best_pref = EQUAL_WEIGHTS;

        let mut prev_alphas: Vec<Preference> = Vec::new();
        let mut alpha = EQUAL_WEIGHTS;
        prev_alphas.push(alpha);
        loop {
            let result = self
                .graph
                .find_shortest_path(
                    dijkstra,
                    0,
                    &[*path.nodes.first().unwrap(), *path.nodes.last().unwrap()],
                    alpha,
                )
                .unwrap();
            let dif = costs_by_alpha(&costs, &alpha)
                - costs_by_alpha(&result.total_dimension_costs, &alpha);

            if dif - accuracy <= 0.0 {
                return Ok(alpha);
            }

            if dif < best_dif {
                best_dif = dif;
                best_pref = alpha;
            }

            let mut cost_dif: Costs = [0.0; EDGE_COST_DIMENSION];

            cost_dif
                .iter_mut()
                .zip(costs.iter().zip(result.total_dimension_costs.iter()))
                .for_each(|(c, (p, r))| *c = r - p);

            self.lp.add_constraint(&cost_dif)?;
            match self.lp.solve()? {
                Some((pref, delta)) => {
                    if best_dif <= -delta + accuracy {
                        return Ok(best_pref);
                    }
                    if prev_alphas.iter().any(|a| a == &pref) {
                        return Ok(best_pref);
                    }
                    alpha = pref;
                    prev_alphas.push(alpha);
                }
                None => {
                    if best_dif < f64::MAX {
                        return Ok(best_pref);
                    } else {
                        panic!("Could not even find a single preference")
                    }
                }
            }
        }
    }

    pub fn calc_representative_preference_for_multiple_paths(
        &mut self,
        dijkstra: &mut Dijkstra,
        paths: &Vec<Path>,
    ) -> MyResult<Preference> {
        self.lp.reset().expect("LP Process could not be reset");
        let mut sum_costs = [0.0; EDGE_COST_DIMENSION];
        for path in paths{
            let costs = path.total_dimension_costs;
            for i in 0..EDGE_COST_DIMENSION{
                sum_costs[i] += costs[i];
            }
        }
        let accuracy = 0.0001;

        let mut best_dif = f64::MAX;
        let mut best_pref = EQUAL_WEIGHTS;

        let mut prev_alphas: Vec<Preference> = Vec::new();
        let mut alpha = EQUAL_WEIGHTS;
        prev_alphas.push(alpha);
        loop {
            let mut sum_optimal_costs = [0.0; EDGE_COST_DIMENSION];
            for path in paths{
                let result = self
                .graph
                .find_shortest_path(
                    dijkstra,
                    0,
                    &[*path.nodes.first().unwrap(), *path.nodes.last().unwrap()],
                    alpha,
                )
                .unwrap();
                for i in 0..EDGE_COST_DIMENSION{
                    sum_optimal_costs[i] += result.total_dimension_costs[i];
                }
            }
            
            let dif = costs_by_alpha(&sum_costs, &alpha)
                - costs_by_alpha(&sum_optimal_costs, &alpha);

            if dif - accuracy <= 0.0 {
                return Ok(alpha);
            }

            if dif < best_dif {
                best_dif = dif;
                best_pref = alpha;
            }

            let mut cost_dif: Costs = [0.0; EDGE_COST_DIMENSION];

            cost_dif
                .iter_mut()
                .zip(sum_costs.iter().zip(sum_optimal_costs.iter()))
                .for_each(|(c, (p, r))| *c = r - p);

            self.lp.add_constraint(&cost_dif)?;
            match self.lp.solve()? {
                Some((pref, delta)) => {
                    if best_dif <= -delta + accuracy {
                        return Ok(best_pref);
                    }
                    if prev_alphas.iter().any(|a| a == &pref) {
                        return Ok(best_pref);
                    }
                    alpha = pref;
                    prev_alphas.push(alpha);
                }
                None => {
                    if best_dif < f64::MAX {
                        return Ok(best_pref);
                    } else {
                        panic!("Could not even find a single preference")
                    }
                }
            }
        }
    }
}

use crate::helpers::{Costs, EQUAL_WEIGHTS};
use crate::MyResult;
use std::io::{BufReader, BufWriter, Read, Write};
use std::process::{Child, Command, Stdio};

pub const F64_SIZE: usize = std::mem::size_of::<f64>();
pub const BUFFER_SIZE: usize = F64_SIZE * EDGE_COST_DIMENSION;
pub const OUTPUT_BUFFER_SIZE: usize = F64_SIZE * (EDGE_COST_DIMENSION + 1);

pub struct LpProcess {
    lp: Child,
}

impl LpProcess {
    pub fn new() -> MyResult<LpProcess> {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        path.push("lp_solver");

        // In case we run tests, we run from the deps directory...
        if !path.exists() {
            path.pop();
            path.pop();
            path.push("lp_solver");
        }

        let lp = Command::new(&path)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        Ok(Self { lp })
    }

    pub fn add_constraint(&mut self, costs: &Costs) -> MyResult<()> {
        let child_stdin = self.lp.stdin.take().unwrap();

        let mut b = BufWriter::new(child_stdin);

        let write_buffer: Vec<_> = costs
            .iter()
            .flat_map(|c| c.to_ne_bytes().iter().copied().collect::<Vec<_>>())
            .collect();

        b.write_all(&[1u8])?;
        b.write_all(&write_buffer)?;
        b.flush()?;

        self.lp.stdin = Some(b.into_inner()?);

        Ok(())
    }

    pub fn reset(&mut self) -> MyResult<()> {
        let child_stdin = self.lp.stdin.as_mut().unwrap();

        let mut b = BufWriter::new(child_stdin);
        b.write_all(&[0u8])?;
        b.flush()?;

        Ok(())
    }

    pub fn solve(&mut self) -> MyResult<Option<(Preference, f64)>> {
        let child_stdin = self.lp.stdin.as_mut().unwrap();

        let mut b = BufWriter::new(child_stdin);
        b.write_all(&[2u8])?;
        b.flush()?;

        let mut buffer = [0u8; OUTPUT_BUFFER_SIZE];
        let child_stdout = self.lp.stdout.as_mut().unwrap();
        let mut r = BufReader::new(child_stdout);
        let mut control_byte = [0u8; 1];

        r.read_exact(&mut control_byte)?;
        match control_byte[0] {
            0 => {
                r.read_exact(&mut buffer)?;
                let mut copy_buff = [0u8; F64_SIZE];
                let result: Vec<_> = buffer
                    .chunks_exact(F64_SIZE)
                    .map(|slice| {
                        copy_buff.copy_from_slice(slice);
                        f64::from_ne_bytes(copy_buff)
                    })
                    .collect();
                let mut pref: Preference = [0.0; EDGE_COST_DIMENSION];
                pref.iter_mut()
                    .zip(result.iter().map(|r| r.max(0.0)))
                    .for_each(|(p, r)| *p = r);
                Ok(Some((pref, *result.last().unwrap())))
            }
            1 => Ok(None),
            x => panic!(format!("Unknown control byte received on main side: {}", x)),
        }
    }
}

#[test]
fn test_calc_representative_preference() {
    use crate::graph::{Edge, Node};

    let nodes = vec![Node::new(0, 0), Node::new(1, 0), Node::new(2, 0)];
    let edges = vec![
        Edge::new(0, 0, 1, [0.0; EDGE_COST_DIMENSION], None),
        Edge::new(1, 1, 2, [10.0; EDGE_COST_DIMENSION], None),
        Edge::new(2, 0, 2, [0.0, 0.0, 0.0, 9.0], None),
        // Edge::new(3, 0, 2, [0.0; EDGE_COST_DIMENSION], None),
        // Edge::new(4, 0, 2, [0.0; EDGE_COST_DIMENSION], None),
    ];

    let graph = Graph::new(nodes, edges);
    let mut dijkstra = Dijkstra::new(&graph);

    let mut lp = LpProcess::new().unwrap();

    let mut estimator = PreferenceEstimator::new(&graph, &mut lp);

    let path = graph
        .find_shortest_path(&mut dijkstra, 0, &[0, 1, 2], EQUAL_WEIGHTS)
        .unwrap();

    let opt = estimator
        .calc_preference(&mut dijkstra, &path, 0, 2)
        .unwrap();

    assert_eq!(None, opt);

    let representative = estimator
        .calc_representative_preference(&mut dijkstra, &path)
        .unwrap();

    assert_eq!([0.0, 0.0, 0.0, 1.0], representative);
}

#[test]
fn test_calc_mixed_representative_preference() {
    use crate::graph::{Edge, Node};

    let nodes = vec![Node::new(0, 0), Node::new(1, 0), Node::new(2, 0)];
    let edges = vec![
        Edge::new(0, 0, 1, [0.0; EDGE_COST_DIMENSION], None),
        Edge::new(1, 1, 2, [10.0; EDGE_COST_DIMENSION], None),
        Edge::new(2, 0, 2, [0.0, 0.0, 12.0, 7.0], None),
        Edge::new(3, 0, 2, [0.0, 0.0, 8.0, 8.0], None),
        Edge::new(4, 0, 2, [0.0, 0.0, 7.0, 12.0], None),
    ];

    let graph = Graph::new(nodes, edges);
    let mut dijkstra = Dijkstra::new(&graph);

    let mut lp = LpProcess::new().unwrap();

    let mut estimator = PreferenceEstimator::new(&graph, &mut lp);

    let path = graph
        .find_shortest_path(&mut dijkstra, 0, &[0, 1, 2], EQUAL_WEIGHTS)
        .unwrap();

    let opt = estimator
        .calc_preference(&mut dijkstra, &path, 0, 2)
        .unwrap();

    assert_eq!(None, opt);

    let representative = estimator
        .calc_representative_preference(&mut dijkstra, &path)
        .unwrap();

    assert_eq!([0.0, 0.0, 0.2, 0.8], representative);
}
