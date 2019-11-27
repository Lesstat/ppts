use criterion::{black_box, criterion_group, criterion_main, Criterion};
use preference_splitting::graph::dijkstra::{find_path, Dijkstra};
use preference_splitting::graphml::read_graphml;
use preference_splitting::EDGE_COST_DIMENSION;

pub fn dijkstra_benchmark(c: &mut Criterion) {
    let graph_data = read_graphml("resources/dijkstra_bench.graphml")
        .expect("could not find graph for dijkstra_bench");

    let mut dijkstra = Dijkstra::new(&graph_data.graph);

    let source = 3;
    let target = 4501;
    let pref = [1.0 / EDGE_COST_DIMENSION as f64; EDGE_COST_DIMENSION];

    c.bench_function("dijkstra", |b| {
        b.iter(|| find_path(&mut dijkstra, &[black_box(source), black_box(target)], pref))
    });
}

criterion_group!(benches, dijkstra_benchmark);
criterion_main!(benches);
