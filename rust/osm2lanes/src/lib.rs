//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

use serde::{Deserialize, Serialize};

mod tags;
pub use self::tags::{Tags, TagsRead, TagsWrite};

mod locale;
pub use self::locale::{DrivingSide, Locale};

mod transform;
pub use self::transform::{
    lanes_to_tags, tags_to_lanes, Lanes, LanesToTagsConfig, RoadError, RoadMsg, RoadWarnings,
    TagsToLanesConfig,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub lanes: Vec<Lane>,
}

/// A single lane
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lane {
    #[serde(rename = "travel")]
    Travel {
        // TODO, we could make this non-optional, but remove the field for designated=foot?
        direction: Option<LaneDirection>,
        designated: LaneDesignated,
    },
    #[serde(rename = "parking")]
    Parking {
        direction: LaneDirection,
        designated: LaneDesignated,
    },
    #[serde(rename = "shoulder")]
    Shoulder,
    // TODO
    #[serde(rename = "separator")]
    Separator,
    // #[serde(rename = "separator")]
    // Construction,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneDirection {
    #[serde(rename = "forward")]
    Forward,
    #[serde(rename = "backward")]
    Backward,
    #[serde(rename = "both")]
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneDesignated {
    // #[serde(rename = "any")]
    // Any,
    #[serde(rename = "foot")]
    Foot,
    #[serde(rename = "bicycle")]
    Bicycle,
    #[serde(rename = "motor_vehicle")]
    Motor,
    #[serde(rename = "bus")]
    Bus,
}

/// Display lane detail as printable characters
pub trait LanePrintable {
    fn as_ascii(&self) -> char;
    fn as_utf8(&self) -> char;
}

impl LanePrintable for Lane {
    fn as_ascii(&self) -> char {
        match self {
            Self::Travel {
                designated: LaneDesignated::Foot,
                ..
            } => 's',
            Self::Travel {
                designated: LaneDesignated::Bicycle,
                ..
            } => 'b',
            Self::Travel {
                designated: LaneDesignated::Motor,
                ..
            } => 'd',
            Self::Travel {
                designated: LaneDesignated::Bus,
                ..
            } => 'B',
            Self::Shoulder => 'S',
            Self::Parking { .. } => 'p',
            Self::Separator => todo!(),
        }
    }
    fn as_utf8(&self) -> char {
        match self {
            Self::Travel {
                designated: LaneDesignated::Foot,
                ..
            } => '🚶',
            Self::Travel {
                designated: LaneDesignated::Bicycle,
                ..
            } => '🚲',
            Self::Travel {
                designated: LaneDesignated::Motor,
                ..
            } => '🚗',
            Self::Travel {
                designated: LaneDesignated::Bus,
                ..
            } => '🚌',
            Self::Shoulder => '🛆',
            Self::Parking { .. } => '🅿',
            Self::Separator => todo!(),
        }
    }
}

impl LanePrintable for LaneDirection {
    fn as_ascii(&self) -> char {
        match self {
            Self::Forward => '^',
            Self::Backward => 'v',
            Self::Both => '|',
        }
    }
    fn as_utf8(&self) -> char {
        match self {
            Self::Forward => '↑',
            Self::Backward => '↓',
            Self::Both => '↕',
        }
    }
}

#[cfg(test)]
mod tests {
    use self::transform::{lanes_to_tags, LanesToTagsConfig};
    use super::*;

    use std::fs::File;
    use std::io::BufReader;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RustTesting {
        Enabled(bool),
        WithOptions { separator: Option<bool> },
    }

    #[derive(Deserialize)]
    struct TestCase {
        // Metadata
        /// The OSM way unique identifier
        way_id: Option<i64>,
        link: Option<String>,
        driving_side: DrivingSide,
        comment: Option<String>,
        description: Option<String>,
        // Data
        tags: Tags,
        output: Vec<Lane>,
        // Skipping
        rust: Option<RustTesting>,
    }

    impl DrivingSide {
        /// Three-letter abbreviation
        const fn as_tla(&self) -> &'static str {
            match self {
                Self::Right => "RHT",
                Self::Left => "LHT",
            }
        }
    }

    impl TestCase {
        fn print(&self) {
            if let Some(description) = self.description.as_ref() {
                println!(
                    "Description: {} ({})",
                    description,
                    self.driving_side.as_tla()
                );
            }
            if self.way_id.is_some() {
                println!(
                    "For input (example from https://www.openstreetmap.org/way/{}) with {}:",
                    self.way_id.unwrap(),
                    self.driving_side.as_tla(),
                );
            } else if self.link.is_some() {
                println!("For input (example from {}):", self.link.as_ref().unwrap());
            }
            if let Some(comment) = self.comment.as_ref() {
                println!("        Comment: {}", comment);
            }
        }

        fn is_enabled(&self) -> bool {
            match self.rust {
                None => true,
                Some(RustTesting::Enabled(b)) => b,
                Some(RustTesting::WithOptions { .. }) => true,
            }
        }

        fn is_separators_tested(&self) -> bool {
            match self.rust {
                None => true,
                Some(RustTesting::Enabled(_)) => unreachable!(),
                Some(RustTesting::WithOptions { separator }) => separator.unwrap_or(true),
            }
        }

        fn expected_road(&self) -> Road {
            Road {
                lanes: self
                    .output
                    .iter()
                    .filter(|lane| self.is_separators_tested() || !matches!(lane, Lane::Separator))
                    .cloned()
                    .collect(),
            }
        }
    }

    fn stringify_lane_types(road: &Road) -> String {
        road.lanes.iter().map(|l| l.as_ascii()).collect()
    }

    fn stringify_directions(road: &Road) -> String {
        road.lanes
            .iter()
            .map(|lane| {
                if let Lane::Travel {
                    direction: Some(direction),
                    ..
                } = lane
                {
                    direction.as_utf8()
                } else {
                    ' '
                }
            })
            .collect()
    }

    #[test]
    fn test_from_data() {
        let tests: Vec<TestCase> =
            serde_json::from_reader(BufReader::new(File::open("../../data/tests.json").unwrap()))
                .expect("invalid json");
        let tests: Vec<TestCase> = tests.into_iter().filter(|test| test.is_enabled()).collect();

        assert!(
            tests.iter().all(|test| {
                let locale = Locale::builder().driving_side(test.driving_side).build();
                let lanes = tags_to_lanes(
                    &test.tags,
                    &locale,
                    &TagsToLanesConfig {
                        error_on_warnings: true,
                    },
                );
                let expected_road = test.expected_road();
                if let Ok(Lanes { lanes, .. }) = lanes {
                    let actual_road = Road { lanes };
                    if actual_road != expected_road {
                        test.print();
                        println!("Got:");
                        println!("    {}", stringify_lane_types(&actual_road));
                        println!("    {}", stringify_directions(&actual_road));
                        println!("Expected:");
                        println!("    {}", stringify_lane_types(&expected_road));
                        println!("    {}", stringify_directions(&expected_road));
                        println!();
                        false
                    } else {
                        true
                    }
                } else {
                    test.print();
                    println!("Expected:");
                    println!("    {}", stringify_lane_types(&expected_road));
                    println!("    {}", stringify_directions(&expected_road));
                    println!("Panicked:");
                    println!("{:#?}", lanes.unwrap_err());
                    println!();
                    false
                }
            }),
            "test_from_data tags_to_lanes failed"
        );
    }

    #[test]
    fn test_roundtrip() {
        let tests: Vec<TestCase> =
            serde_json::from_reader(BufReader::new(File::open("../../data/tests.json").unwrap()))
                .unwrap();
        let tests: Vec<TestCase> = tests.into_iter().filter(|test| test.is_enabled()).collect();

        assert!(
            tests.iter().all(|test| {
                let locale = Locale::builder().driving_side(test.driving_side).build();
                let input_road = test.expected_road();
                let tags = lanes_to_tags(
                    &test.output,
                    &locale,
                    &LanesToTagsConfig {
                        check_roundtrip: false,
                    },
                )
                .unwrap();
                let output_lanes = tags_to_lanes(
                    &tags,
                    &locale,
                    &TagsToLanesConfig {
                        error_on_warnings: false,
                    },
                )
                .unwrap();
                let output_road = Road {
                    lanes: output_lanes.lanes,
                };
                if input_road != output_road {
                    test.print();
                    println!("From:");
                    println!("    {}", stringify_lane_types(&input_road));
                    println!("    {}", stringify_directions(&input_road));
                    println!("Normalized OSM tags:");
                    for (k, v) in tags.map() {
                        println!("    {} = {}", k, v);
                    }
                    println!("Got:");
                    println!("    {}", stringify_lane_types(&output_road));
                    println!("    {}", stringify_directions(&output_road));
                    println!();
                    false
                } else {
                    true
                }
            }),
            "test_roundtrip lanes_to_tags failed"
        );
    }
}
