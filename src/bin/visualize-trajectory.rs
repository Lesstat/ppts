use preference_splitting::geojson::read_geojson_map;
use preference_splitting::statistics::{
    read_representative_results, read_splitting_results, ExperimentResults,
    RepresentativeAlphaResult, SplittingStatistics,
};
use preference_splitting::trajectories::{read_trajectories, Trajectory};
use preference_splitting::MyResult;

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use geojson::{Feature, FeatureCollection, Geometry};
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
    Preferences,
    /// Visualize the trajectory alone
    TrajectoryOnly {
        #[structopt(default_value = "#000")]
        color: String,
    },
    /// Visualize decomposition windows
    Windows {
        #[structopt(default_value = "#000")]
        trajectory_color: String,
        #[structopt(default_value = "#00f")]
        nos_color: String,
    },
    #[allow(dead_code)]
    Representative {
        #[structopt(default_value = "#000")]
        trajectory_color: String,
        #[structopt(default_value = "#00f")]
        representatvive_color: String,
    },
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
        Style::Preferences => visualize_trajectories(
            &trajectory,
            &stats.splitting().removed_self_loop_indices,
            &mut PreferenceVis::new(stats.splitting()),
            &geojson_map,
        ),

        Style::TrajectoryOnly { color } => visualize_trajectories(&trajectory,&[], &mut OneColor(color), &geojson_map),
        Style::Windows {
            trajectory_color,
            nos_color,
        } => visualize_trajectories(
            &trajectory,
            &stats.splitting().removed_self_loop_indices,
            &mut DecompositionWindowsStyle::new(stats.splitting(), trajectory_color, nos_color),
            &geojson_map,
        ),
        Style::Representative {
            ..
            // trajectory_color,
            // representatvive_color,
        } => {
            let _ = stats.representative();
            todo!(
                r#"Make this happen, first vis trajectory in traj_color,
                   then read graph, then calc repr, then vis repr in repr_color"#
            );
        }
    };

    let outfile = format!("geojson_trajectory_{}.json", trajectory_id);

    println!("saving into file {}", outfile);

    let file = std::fs::File::create(outfile)?;
    let mut file = std::io::BufWriter::new(file);

    file.write_all(serde_json::to_string(&feature_collection)?.as_bytes())?;
    Ok(())
}

fn load_results(style: &Style, path: PathBuf) -> Result<Results, Box<dyn std::error::Error>> {
    if let Style::Representative { .. } = style {
        Ok(Results::Representative(read_representative_results(path)?))
    } else {
        Ok(Results::Splitting(read_splitting_results(path)?))
    }
}

trait EdgeStyle {
    fn properties(&mut self, index: u32)
        -> Option<serde_json::map::Map<String, serde_json::Value>>;
}

struct OneColor(String);

impl EdgeStyle for OneColor {
    fn properties(&mut self, _: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        use serde_json::{Number, Value};

        let mut map = serde_json::map::Map::new();

        map.insert("stroke".to_owned(), Value::String(self.0.clone()));

        map.insert(
            "stroke-opacity".to_owned(),
            Value::Number(Number::from_f64(0.7).unwrap()),
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
}

impl<'a> DecompositionWindowsStyle<'a> {
    pub fn new(
        splitting: &'a SplittingStatistics,
        traj_color: String,
        nos_color: String,
    ) -> DecompositionWindowsStyle {
        DecompositionWindowsStyle {
            splitting,
            traj_color,
            nos_color,
        }
    }
}

impl<'a> EdgeStyle for DecompositionWindowsStyle<'a> {
    fn properties(&mut self, index: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        use serde_json::{Number, Value};

        let mut map = serde_json::map::Map::new();

        let color = if let Some(non_opt_subpath) = &self.splitting.non_opt_subpaths {
            if non_opt_subpath
                .decomposition_windows
                .iter()
                .any(|w| w.0 <= index && index < w.1)
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
            Value::Number(Number::from_f64(0.7).unwrap()),
        );

        map.insert(
            "stroke-width".to_owned(),
            Value::Number(Number::from_f64(5.0).unwrap()),
        );

        Some(map)
    }
}

struct PreferenceVis<'a> {
    splitting: &'a SplittingStatistics,
}

impl<'a> PreferenceVis<'a> {
    fn new(splitting: &'a SplittingStatistics) -> Self {
        Self { splitting }
    }
}

impl<'a> EdgeStyle for PreferenceVis<'a> {
    fn properties(&mut self, index: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        use serde_json::{Number, Value};

        let colors = [
            // "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628", "#f781bf",
            // "#999999",
            "#a6611a", "#dfc27d", "#984ea3", "#80cdc1", "#018571",
        ];

        let index = match self.splitting.cuts.binary_search(&index) {
            Ok(i) => i,
            Err(i) => i,
        } % colors.len();

        let mut map = serde_json::map::Map::new();

        map.insert("stroke".to_owned(), Value::String(colors[index].to_owned()));

        map.insert(
            "stroke-opacity".to_owned(),
            Value::Number(Number::from_f64(0.7).unwrap()),
        );

        map.insert(
            "stroke-width".to_owned(),
            Value::Number(Number::from_f64(5.0).unwrap()),
        );
        map.insert(
            "preference".to_owned(),
            Value::String(format!("{:?}", self.splitting.preferences[index])),
        );

        Some(map)
    }
}

fn visualize_trajectories(
    trajectory: &Trajectory,
    removed_self_loops: &[u32],
    style: &mut dyn EdgeStyle,
    geojson_map: &HashMap<i64, Geometry>,
) -> FeatureCollection {
    let line_strings = trajectory
        .path
        .iter()
        .map(|e| geojson_map[e].clone())
        .collect::<Vec<_>>();

    let mut features: Vec<Feature> = Vec::new();
    for (i, geom) in line_strings.into_iter().enumerate() {
        let self_loop_count = removed_self_loops
            .iter()
            .take_while(|l| **l <= i as u32)
            .count();
        let i = (i - self_loop_count) as u32;

        if let Some(marker) = make_break_marker(&trajectory.trip_id, i, &geom) {
            features.push(marker);
        }

        features.push(Feature {
            id: None,
            bbox: None,
            foreign_members: None,
            properties: style.properties(i as u32),
            geometry: Some(geom),
        })
    }

    let start_marker_pos = match &features[0].geometry.as_ref().unwrap().value {
        geojson::Value::LineString(line) => &line[0],
        _ => panic!("edge is not a linestring I don't know what to do"),
    };

    let start_marker = make_marker(start_marker_pos.to_vec(), "#000", "s");

    let end_marker_pos = match &features.last().unwrap().geometry.as_ref().unwrap().value {
        geojson::Value::LineString(line) => &line[0],
        _ => panic!("edge is not a linestring I don't know what to do"),
    };

    let end_marker = make_marker(end_marker_pos.to_vec(), "#000", "t");

    features.insert(0, start_marker);
    features.push(end_marker);

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn make_marker(pos: Vec<f64>, color: &str, symbol: &str) -> Feature {
    let mut map = serde_json::map::Map::new();
    map.insert(
        "marker-color".to_string(),
        serde_json::Value::String(color.to_string()),
    );

    map.insert(
        "marker-symbol".to_string(),
        serde_json::Value::String(symbol.to_string()),
    );
    Feature {
        bbox: None,
        geometry: Some(Geometry {
            bbox: None,
            value: geojson::Value::Point(pos),
            foreign_members: None,
        }),
        id: None,
        properties: Some(map),
        foreign_members: None,
    }
}

fn make_break_marker(
    trip_id: &[(Option<u32>, u32)],
    index: u32,
    geom: &Geometry,
) -> Option<Feature> {
    if index > 0 && trip_id.iter().any(|id| id.1 == index) {
        let break_marker_pos = match &geom.value {
            geojson::Value::LineString(line) => &line[0],
            _ => panic!("edge is not a linestring I don't know what to do"),
        };
        let break_marker = make_marker(break_marker_pos.to_vec(), "#FBFF45", "b");
        Some(break_marker)
    } else {
        None
    }
}
