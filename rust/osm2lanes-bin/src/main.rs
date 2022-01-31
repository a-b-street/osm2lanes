use clap::Parser;

use osm2lanes::overpass::get_way;
use osm2lanes::{tags_to_lanes, Locale, TagsToLanesConfig};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Way ID
    id: u64,
}

fn main() {
    let args = Args::parse();
    let tags = get_way(args.id);
    println!("{:#?}", tags);
    let locale = Locale::builder().build();
    let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
    println!("{:#?}", lanes);
}
