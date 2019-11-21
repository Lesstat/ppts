use preference_splitting::helpers::Preference;
use preference_splitting::statistics::read_splitting_results;
use preference_splitting::trajectories::read_trajectories;
use preference_splitting::MyResult;

use std::collections::HashMap;
use std::env::args;
use std::io::Write;

use geojson::{Feature, FeatureCollection, Geometry, Value};

fn main() -> MyResult<()> {
    let args: Vec<_> = args().collect();

    if args.len() != 5 {
        println!("Not correct amount of arguments");
        println!(
            "{} <trajectories.json> <splitting_results.json> <geojson.json> <trajectory-id>",
            args[0]
        );
        return Ok(());
    }
    let trajectory_file = &args[1];
    let splitting_results_file = &args[2];
    let geojson_file = &args[3];
    let trajectory_id: i64 = args[4].parse()?;

    println!("reading input files");

    let trajectories = read_trajectories(trajectory_file)?;
    let splitting_results = read_splitting_results(splitting_results_file)?;
    let geojson_map = read_geojson_map(geojson_file)?;

    println!("searching trajectory {}", trajectory_id);

    let trajectory = trajectories
        .iter()
        .find(|t| t.trip_id == trajectory_id)
        .expect("could not find trajectory in trajectories file.");

    let splitting = splitting_results
        .results
        .iter()
        .find(|s| s.trip_id == trajectory_id)
        .expect("could not find trajectory in splitting results.");

    println!("creating geojson");

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
                properties: create_properties(i, &splitting.preferences[i]),
                geometry: Some(geometry),
            }
        })
        .collect();

    let feature_collection = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };

    let outfile = format!("geojson_trajectory_{}.json", trajectory_id);

    println!("saving into file {}", outfile);

    let file = std::fs::File::create(outfile)?;
    let mut file = std::io::BufWriter::new(file);

    file.write_all(serde_json::to_string(&feature_collection)?.as_bytes())?;
    Ok(())
}

fn read_geojson_map<P: AsRef<std::path::Path>>(path: P) -> MyResult<HashMap<i64, Geometry>> {
    let file = std::fs::File::open(path)?;
    let file = std::io::BufReader::new(file);
    let map: HashMap<i64, String> = serde_json::from_reader(file)?;

    Ok(map
        .iter()
        .map(|(&i, s)| (i, serde_json::from_str(s).expect("could not parse geojson")))
        .collect())
}

fn create_properties(
    i: usize,
    p: &Preference,
) -> Option<serde_json::map::Map<String, serde_json::Value>> {
    let colors = [
        "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628", "#f781bf",
        "#999999",
    ];

    let index = i % colors.len();

    let mut map = serde_json::map::Map::new();

    map.insert(
        "stroke".to_owned(),
        serde_json::Value::String(colors[index].to_owned()),
    );

    map.insert(
        "preference".to_owned(),
        serde_json::Value::String(format!("{:?}", p).to_owned()),
    );

    Some(map)
}
