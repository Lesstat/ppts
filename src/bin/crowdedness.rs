use geojson::Value;
use osmpbfreader::{OsmObj, OsmPbfReader};

use preference_splitting::geojson::read_geojson_map;
use preference_splitting::{MyError, MyResult};

use std::collections::HashMap;
use std::env::args;

#[derive(Debug)]
struct CrowdednessGrid {
    min_lat: f64,
    min_lng: f64,
    max_lat: f64,
    max_lng: f64,
    grid_size: usize,
    grid: Vec<usize>,
}

fn main() -> MyResult<()> {
    let args: Vec<_> = args().collect();

    if args.len() != 4 {
        println!("usage: {} <pbf-file> <geojson.json> <grid-size>", args[0]);
        return Err(Box::new(MyError::InvalidTrajectories));
    }

    let pbf_path = &args[1];
    let geojson_path = &args[2];
    let grid_size: usize = args[3].parse()?;

    let geojson_map = read_geojson_map(geojson_path)?;

    let mut crowdedness_grid = CrowdednessGrid::new(grid_size);

    println!("Determining relevant bounding box from street network");
    for geometry in geojson_map.values() {
        match geometry.value {
            Value::LineString(ref pos) => pos
                .iter()
                .for_each(|p| crowdedness_grid.add_graph_point(p[1], p[0])),
            _ => println!("not matched geometry value"),
        }
    }

    let pbf_file = std::fs::File::open(pbf_path)?;
    let mut pbf = OsmPbfReader::new(pbf_file);

    println!("Collecting nodes from PBF file");
    pbf.iter()
        .filter_map(|obj| {
            if let Ok(OsmObj::Node(n)) = obj {
                Some(n)
            } else {
                None
            }
        })
        .for_each(|n| crowdedness_grid.add_crowd_point(n.lat(), n.lon()));

    println!("Calculate crowdedness per edge");
    let crowdedness_assignment: HashMap<_, usize> = geojson_map
        .iter()
        .map(|(i, l)| match l.value {
            Value::LineString(ref pos) => (
                i,
                pos.iter()
                    .map(|p| crowdedness_grid.crowdedness(p[1], p[0]))
                    .sum(),
            ),
            _ => (i, 0),
        })
        .collect();

    println!("Normalize per edge costs");
    let max_cost = *crowdedness_assignment
        .values()
        .max()
        .expect("There are no edges");
    let crowdedness_assignment: HashMap<_, Vec<f64>> = crowdedness_assignment
        .into_iter()
        .map(|(i, v)| (i, vec![v as f64 / max_cost as f64]))
        .collect();

    let outfile = std::fs::File::create("crowdedness.json")?;
    let writer = std::io::BufWriter::new(outfile);

    serde_json::to_writer(writer, &crowdedness_assignment)?;

    Ok(())
}

impl CrowdednessGrid {
    fn new(grid_size: usize) -> CrowdednessGrid {
        let grid = vec![0; grid_size * grid_size];

        CrowdednessGrid {
            min_lat: 180.0,
            min_lng: 180.0,
            max_lat: -180.0,
            max_lng: -180.0,
            grid_size,
            grid,
        }
    }

    fn add_graph_point(&mut self, lat: f64, lng: f64) {
        self.min_lat = lat.min(self.min_lat);
        self.min_lng = lng.min(self.min_lng);

        self.max_lat = lat.max(self.max_lat);
        self.max_lng = lng.max(self.max_lng);
    }

    fn add_crowd_point(&mut self, lat: f64, lng: f64) {
        if !self.contains(lat, lng) {
            return;
        }
        let index = self.get_index(lat, lng);
        self.grid[index] += 1;
    }

    fn get_index(&self, lat: f64, lng: f64) -> usize {
        let lat_dist = self.max_lat - self.min_lat;
        let lat_step = lat_dist / self.grid_size as f64;
        let lat_offset = lat - self.min_lat;
        let lat_index = (lat_offset / lat_step).floor() as usize;

        let lat_index = if lat_index == self.grid_size {
            lat_index - 1
        } else {
            lat_index
        };

        let lng_dist = self.max_lng - self.min_lng;
        let lng_step = lng_dist / self.grid_size as f64;
        let lng_offset = lng - self.min_lng;
        let lng_index = (lng_offset / lng_step).floor() as usize;

        let lng_index = if lng_index == self.grid_size {
            lng_index - 1
        } else {
            lng_index
        };

        lat_index + lng_index * self.grid_size
    }

    fn crowdedness(&self, lat: f64, lng: f64) -> usize {
        let index = self.get_index(lat, lng);
        if index > self.grid.len() {
            dbg!((lat, lng));
        }
        self.grid[index]
    }

    fn contains(&self, lat: f64, lng: f64) -> bool {
        self.min_lat <= lat && lat <= self.max_lat && self.min_lng <= lng && lng <= self.max_lng
    }
}

#[test]
fn test_bbox_init() {
    let mut grid = CrowdednessGrid::new(10);

    grid.add_graph_point(10.0, 9.0);
    grid.add_graph_point(8.0, 11.0);

    assert_eq!(8.0, grid.min_lat);
    assert_eq!(10.0, grid.max_lat);

    assert_eq!(9.0, grid.min_lng);
    assert_eq!(11.0, grid.max_lng);
}

#[test]
fn test_index_calculation() {
    let mut grid = CrowdednessGrid::new(10);

    grid.add_graph_point(10.0, 9.0);
    grid.add_graph_point(8.0, 11.0);

    assert_eq!(0, grid.get_index(8.0, 9.0));
    assert_eq!(1, grid.get_index(8.21, 9.0));
    assert_eq!(10, grid.get_index(8.0, 9.21));
    assert_eq!(11, grid.get_index(8.21, 9.21));
    assert_eq!(99, grid.get_index(10.0, 11.0));
}
