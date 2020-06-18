use preference_splitting::geojson::read_geojson_map;
use preference_splitting::helpers::Preference;
use preference_splitting::statistics::{
    read_representative_results, read_splitting_results, ExperimentResults,
    RepresentativeAlphaResult, SplittingStatistics,
};
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
    /// File containing results of an experiment
    results_file: PathBuf,
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
    /// Visualize the trajectory alone
    TrajectoryOnly,
    /// Visualize decomposition windows
    Windows,
}

enum Results {
    Splitting(ExperimentResults<SplittingStatistics>),
    Representative(ExperimentResults<RepresentativeAlphaResult>),
}

impl Results {
    fn find_trip(&self, trajectory_id: i64) -> ResultEntry {
        match self {
            Results::Splitting(res) => {
                let trip_res = res
                    .results
                    .iter()
                    .find(|s| s.trip_id[0].0.unwrap_or(u32::MAX) as i64 == trajectory_id)
                    .expect("could not find trajectory in splitting results.");
                ResultEntry::Splitting(trip_res)
            }
            Results::Representative(res) => {
                let trip_res = res
                    .results
                    .iter()
                    .find(|s| s.trip_id[0].0.unwrap_or(u32::MAX) as i64 == trajectory_id)
                    .expect("could not find trajectory in splitting results.");
                ResultEntry::Representative(trip_res)
            }
        }
    }
}

enum ResultEntry<'a> {
    Splitting(&'a SplittingStatistics),
    Representative(&'a RepresentativeAlphaResult),
}

impl<'a> ResultEntry<'a> {
    fn splitting(&self) -> &'a SplittingStatistics {
        match self {
            ResultEntry::Splitting(s) => s,
            _ => panic!("Wrong type of statistic"),
        }
    }

    fn representative(&self) -> &'a RepresentativeAlphaResult {
        match self {
            ResultEntry::Representative(r) => r,
            _ => panic!("Wrong type of statistic"),
        }
    }
}

fn main() -> MyResult<()> {
    let Opt {
        style,
        trajectory_file,
        results_file,
        geojson_file,
        trajectory_id,
    } = Opt::from_args();

    println!("reading input files");

    let trajectories = read_trajectories(trajectory_file)?;
    let results = load_results(&style, results_file)
        .expect("Could not read results. Do they fit your visualization style?");
    let geojson_map = read_geojson_map(geojson_file)?;

    println!("searching trajectory {}", trajectory_id);

    let trajectory = trajectories
        .iter()
        .find(|t| t.trip_id[0].0.unwrap_or(u32::MAX) as i64 == trajectory_id)
        .expect("could not find trajectory in trajectories file.");

    let stats = results.find_trip(trajectory_id);

    println!("creating geojson");

    let feature_collection = match style {
        Style::Cuts => visualize_cuts(&trajectory, &stats.splitting(), &geojson_map),
        Style::NonOpts => visualize_snops(&trajectory, &stats.splitting(), &geojson_map),
        Style::TrajectoryOnly => {
            visualize_trajectories(&trajectory, &mut OneColor("#000".to_owned()), &geojson_map)
        }
        Style::Windows => visualize_trajectories(
            &trajectory,
            &mut DecompositionWindowsStyle::new(stats.splitting()),
            &geojson_map,
        ),
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

fn load_results(_style: &Style, path: PathBuf) -> Result<Results, Box<dyn std::error::Error>> {
    Ok(Results::Splitting(read_splitting_results(path)?))
}

trait SegmentStyle {
    fn properties(&mut self) -> Option<serde_json::map::Map<String, serde_json::Value>>;
}

struct OneColor(String);

impl SegmentStyle for OneColor {
    fn properties(&mut self) -> Option<serde_json::Map<String, serde_json::Value>> {
        use serde_json::{Number, Value};

        let mut map = serde_json::map::Map::new();

        map.insert("stroke".to_owned(), Value::String(self.0.clone()));

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
}

struct DecompositionWindowsStyle<'a> {
    splitting: &'a SplittingStatistics,
    traj_color: String,
    nos_color: String,
    path_counter: u32,
}

impl<'a> DecompositionWindowsStyle<'a> {
    pub fn new(splitting: &'a SplittingStatistics) -> DecompositionWindowsStyle {
        DecompositionWindowsStyle {
            splitting,
            traj_color: "#000".to_string(),
            nos_color: "#0000ff".to_string(),
            path_counter: 0,
        }
    }
}

impl<'a> SegmentStyle for DecompositionWindowsStyle<'a> {
    fn properties(&mut self) -> Option<serde_json::Map<String, serde_json::Value>> {
        use serde_json::{Number, Value};

        let mut map = serde_json::map::Map::new();

        let color = if let Some(non_opt_subpath) = &self.splitting.non_opt_subpaths {
            if non_opt_subpath
                .decomposition_windows
                .iter()
                .any(|w| w.0 <= self.path_counter && self.path_counter < w.1)
            {
                self.nos_color.clone()
            } else {
                self.traj_color.clone()
            }
        } else {
            panic!(
                "Trip {:?} has no decomposition data",
                self.splitting.trip_id
            );
        };

        map.insert("stroke".to_owned(), Value::String(color));

        map.insert(
            "stroke-opacity".to_owned(),
            Value::Number(Number::from_f64(0.5).unwrap()),
        );

        map.insert(
            "stroke-width".to_owned(),
            Value::Number(Number::from_f64(5.0).unwrap()),
        );

        self.path_counter += 1;
        Some(map)
    }
}

fn visualize_trajectories(
    trajectory: &Trajectory,
    style: &mut dyn SegmentStyle,
    geojson_map: &HashMap<i64, Geometry>,
) -> FeatureCollection {
    let features = trajectory
        .path
        .iter()
        .map(|e| geojson_map[e].clone())
        .map(|geom| Feature {
            id: None,
            bbox: None,
            foreign_members: None,
            properties: style.properties(),
            geometry: Some(geom),
        })
        .collect();

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}
