use std::fs::File;
use std::io::{BufReader, Write};

use osm2lanes::{tags_to_lanes, Locale, Tags, TagsToLanesConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        panic!("Usage: osm2lanes INPUT_FILE OUTPUT_FILE");
    }

    let tags: Tags =
        serde_json::from_reader(BufReader::new(File::open(&args[1]).unwrap())).unwrap();
    let locale = Locale::builder().build();
    let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
    let mut file = File::create(&args[2]).unwrap();
    writeln!(file, "{}", serde_json::to_string_pretty(&lanes).unwrap()).unwrap();
}
