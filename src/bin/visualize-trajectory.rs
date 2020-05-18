use preference_splitting::geojson::read_geojson_map;
use preference_splitting::helpers::Preference;
use preference_splitting::statistics::{read_splitting_results, SplittingStatistics};
use preference_splitting::trajectories::{read_trajectories, Trajectory};
use preference_splitting::MyResult;

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use geojson::{Feature, FeatureCollection, Geometry, Value};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// Style of visualization
    #[structopt(subcommand)]
    style: Style,
    /// File containing all trajectories
    trajectory_file: PathBuf,
    /// File containing results of the MOPS algorithm
    splitting_results_file: PathBuf,
    /// File containing the geometry of the graph as geojson
    geojson_file: PathBuf,
    /// ID of the trajectory to visualize
    trajectory_id: i64,
}

#[derive(StructOpt)]
enum Style {
    /// Visualizes the different optimal segments in different colors
    Cuts,
    /// Visualizes the non-optimal sub paths
    NonOpts,
}

fn main() -> MyResult<()> {
    let Opt {
        style,
        trajectory_file,
        splitting_results_file,
        geojson_file,
        trajectory_id,
    } = Opt::from_args();

    println!("reading input files");

    let trajectories = read_trajectories(trajectory_file)?;
    let splitting_results = read_splitting_results(splitting_results_file)?;
    let geojson_map = read_geojson_map(geojson_file)?;

    println!("searching trajectory {}", trajectory_id);

    let trajectory = trajectories
        .iter()
        .find(|t| t.trip_id[0].0.unwrap_or(u32::MAX) as i64 == trajectory_id)
        .expect("could not find trajectory in trajectories file.");

    let splitting = splitting_results
        .results
        .iter()
        .find(|s| s.trip_id[0].0.unwrap_or(u32::MAX) as i64 == trajectory_id)
        .expect("could not find trajectory in splitting results.");

    println!("creating geojson");

    let feature_collection = match style {
        Style::Cuts => visualize_cuts(&trajectory, &splitting, &geojson_map),
        Style::NonOpts => visualize_snops(&trajectory, &splitting, &geojson_map),
    };

    let outfile = format!("geojson_trajectory_{}.json", trajectory_id);

    println!("saving into file {}", outfile);

    let file = std::fs::File::create(outfile)?;
    let mut file = std::io::BufWriter::new(file);

    file.write_all(serde_json::to_string(&feature_collection)?.as_bytes())?;
    Ok(())
}

fn visualize_cuts(
    trajectory: &Trajectory,
    splitting: &SplittingStatistics,
    geojson_map: &HashMap<i64, Geometry>,
) -> FeatureCollection {
    let mut last_cut = 0;
    let features = splitting
        .cuts
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let line_strings: Vec<Geometry> = trajectory.path[last_cut..c]
                .iter()
                .map(|e| geojson_map[e].clone())
                .collect();
            last_cut = c;

            let geo_collection = Value::GeometryCollection(line_strings);
            let geometry = Geometry {
                bbox: None,
                foreign_members: None,
                value: geo_collection,
            };

            Feature {
                id: None,
                bbox: None,
                foreign_members: None,
                properties: create_cut_properties(i, &splitting.preferences[i]),
                geometry: Some(geometry),
            }
        })
        .collect();

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn create_cut_properties(
    i: usize,
    p: &Preference,
) -> Option<serde_json::map::Map<String, serde_json::Value>> {
    use serde_json::{Number, Value};
    let colors = [
        "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628", "#f781bf",
        "#999999",
    ];

    let index = i % colors.len();

    let mut map = serde_json::map::Map::new();

    map.insert("stroke".to_owned(), Value::String(colors[index].to_owned()));

    map.insert(
        "stroke-opacity".to_owned(),
        Value::Number(Number::from_f64(0.5).unwrap()),
    );

    map.insert(
        "stroke-width".to_owned(),
        Value::Number(Number::from_f64(5.0).unwrap()),
    );
    map.insert("preference".to_owned(), Value::String(format!("{:?}", p)));

    Some(map)
}

fn visualize_snops(
    trajectory: &Trajectory,
    splitting: &SplittingStatistics,
    geojson_map: &HashMap<i64, Geometry>,
) -> FeatureCollection {
    let mut last = 0;
    let mut features: Vec<_> = splitting
        .non_opt_subpaths
        .as_ref()
        .expect("SNOPs data not present in Results file")
        .non_opt_subpaths
        .iter()
        .flat_map(|(start, end)| {
            let line_strings = trajectory.path[last..*start]
                .iter()
                .map(|e| geojson_map[e].clone())
                .collect();

            let geo_collection = Value::GeometryCollection(line_strings);
            let geometry = Geometry {
                bbox: None,
                foreign_members: None,
                value: geo_collection,
            };

            let opt = Feature {
                id: None,
                bbox: None,
                foreign_members: None,
                properties: create_non_opt_properties(false),
                geometry: Some(geometry),
            };

            let line_strings = trajectory.path[*start..*end]
                .iter()
                .map(|e| geojson_map[e].clone())
                .collect();

            let geo_collection = Value::GeometryCollection(line_strings);
            let geometry = Geometry {
                bbox: None,
                foreign_members: None,
                value: geo_collection,
            };

            let non_opt = Feature {
                id: None,
                bbox: None,
                foreign_members: None,
                properties: create_non_opt_properties(true),
                geometry: Some(geometry),
            };

            last = *end;

            vec![opt, non_opt]
        })
        .collect();

    let line_strings = trajectory.path[last as usize..trajectory.path.len()]
        .iter()
        .map(|e| geojson_map[e].clone())
        .collect();

    let geo_collection = Value::GeometryCollection(line_strings);
    let geometry = Geometry {
        bbox: None,
        foreign_members: None,
        value: geo_collection,
    };

    let opt = Feature {
        id: None,
        bbox: None,
        foreign_members: None,
        properties: create_non_opt_properties(false),
        geometry: Some(geometry),
    };

    features.push(opt);

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn create_non_opt_properties(
    non_opt: bool,
) -> Option<serde_json::map::Map<String, serde_json::Value>> {
    use serde_json::{Number, Value};
    let mut map = serde_json::map::Map::new();

    let color = if non_opt { "#ff0000" } else { "#000000" };

    map.insert("stroke".to_owned(), Value::String(color.to_owned()));

    map.insert(
        "stroke-opacity".to_owned(),
        Value::Number(Number::from_f64(0.5).unwrap()),
    );

    map.insert(
        "stroke-width".to_owned(),
        Value::Number(Number::from_f64(5.0).unwrap()),
    );

    Some(map)
}
