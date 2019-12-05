use crate::MyResult;

use geojson::Geometry;
use std::collections::HashMap;

pub fn read_geojson_map<P: AsRef<std::path::Path>>(path: P) -> MyResult<HashMap<i64, Geometry>> {
    let file = std::fs::File::open(path)?;
    let file = std::io::BufReader::new(file);
    let map: HashMap<i64, String> = serde_json::from_reader(file)?;

    Ok(map
        .iter()
        .map(|(&i, s)| (i, serde_json::from_str(s).expect("could not parse geojson")))
        .collect())
}
