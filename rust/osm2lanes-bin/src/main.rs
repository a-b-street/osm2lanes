use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use clap::{AppSettings, Parser, Subcommand};
use futures::executor::block_on;
use osm2lanes::overpass::get_way;
use osm2lanes::tags::Tags;
use osm2lanes::{tags_to_lanes, Locale, TagsToLanesConfig};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(name = "osm2lanes", author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Retrieve lanes given OSM way ID
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Way {
        /// Way ID
        #[clap(required = true)]
        id: u64,
    },
    /// Convert OSM way tags to lanes
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Convert {
        /// JSON of OSM Tags
        #[clap(required = true, parse(from_os_str))]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    flexi_logger::Logger::try_with_env()
        .unwrap()
        .start()
        .unwrap();
    let args = Cli::parse();
    match &args.command {
        Command::Way { id } => {
            let tags = block_on(get_way(*id)).unwrap();
            log::info!("{:#?}", tags);
            let locale = Locale::builder().build();
            let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
            println!("{}", serde_json::to_string_pretty(&lanes).unwrap());
        }
        Command::Convert { path } => {
            let tags: Tags =
                serde_json::from_reader(BufReader::new(File::open(path).unwrap())).unwrap();
            let locale = Locale::builder().build();
            let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
            println!("{}", serde_json::to_string_pretty(&lanes).unwrap());
        }
    }
}
