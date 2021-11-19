use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};

use serde::Deserialize;

use osm2lanes::{get_lane_specs_ltr, Config, DrivingSide};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        panic!("Usage: osm2lanes INPUT_FILE OUTPUT_FILE");
    }

    let input: Tags =
        serde_json::from_reader(BufReader::new(File::open(&args[1]).unwrap())).unwrap();
    let lanes = get_lane_specs_ltr(
        input.0,
        &Config {
            driving_side: DrivingSide::Right,
            inferred_sidewalks: true,
        },
    );
    let mut file = File::create(&args[2]).unwrap();
    writeln!(file, "{}", serde_json::to_string_pretty(&lanes).unwrap()).unwrap();
}

#[derive(Deserialize)]
struct Tags(BTreeMap<String, String>);
