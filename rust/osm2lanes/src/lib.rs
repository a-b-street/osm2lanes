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
    get_lane_specs_ltr, get_lane_specs_ltr_with_warnings, lanes_to_tags, LaneSpecWarnings, Lanes,
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
    #[serde(rename = "none")]
    None,
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
            Self::Shoulder => 'ˢ',
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
            Self::None => '-',
        }
    }
    fn as_utf8(&self) -> char {
        match self {
            Self::Forward => '↑',
            Self::Backward => '↓',
            Self::Both => '↕',
            Self::None => '—',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;
    use std::io::BufReader;

    #[derive(Deserialize)]
    struct TestCase {
        // Metadata
        /// The OSM way unique identifier
        way_id: Option<i64>,
        link: Option<String>,
        driving_side: DrivingSide,
        #[serde(rename = "skip_rust")]
        skip: Option<bool>,
        comment: Option<String>,
        description: Option<String>,
        // Data
        tags: Tags,
        output: Vec<Lane>,
    }

    impl TestCase {
        fn print(&self) {
            if !self.way_id.is_none() {
                println!(
                    "For input (example from https://www.openstreetmap.org/way/{}) with {}:",
                    self.way_id.unwrap(),
                    self.driving_side.to_tla(),
                );
            } else if !self.link.is_none() {
                println!("For input (example from {}):", self.link.as_ref().unwrap());
            }
            if let Some(comment) = self.comment.as_ref() {
                println!("        Comment: {}", comment);
            }
            if let Some(description) = self.description.as_ref() {
                println!("        Description: {}", description);
            }
            for (k, v) in self.tags.map() {
                println!("    {} = {}", k, v);
            }
        }
    }

    impl DrivingSide {
        /// Three-letter abbreviation
        const fn to_tla(&self) -> &'static str {
            match self {
                Self::Right => "RHT",
                Self::Left => "LHT",
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
                .unwrap();

        let mut ok = true;
        for test in tests.iter().filter(|test| {
            !test
                .output
                .iter()
                .any(|lane| matches!(lane, Lane::Separator))
        }) {
            if !test.skip.is_none() && test.skip.unwrap() {
                continue;
            }
            let lc = Locale::builder().driving_side(test.driving_side).build();
            let lanes = get_lane_specs_ltr(&test.tags, &lc);
            let expected_road = Road {
                lanes: test.output.clone(),
            };
            if let Ok(actual_road) = lanes {
                if actual_road != expected_road {
                    ok = false;
                    test.print();
                    println!("Got:");
                    println!("    {}", stringify_lane_types(&actual_road));
                    println!("    {}", stringify_directions(&actual_road));
                    println!("Expected:");
                    println!("    {}", stringify_lane_types(&expected_road));
                    println!("    {}", stringify_directions(&expected_road));
                    println!();
                }
            } else {
                ok = false;
                test.print();
                println!("Expected:");
                println!("    {}", stringify_lane_types(&expected_road));
                println!("    {}", stringify_directions(&expected_road));
                println!("Panicked:");
                println!("{:#?}", lanes.unwrap_err());
                println!();
            }
        }
        assert!(ok);
    }

    #[test]
    fn test_roundtrip() {
        let tests: Vec<TestCase> =
            serde_json::from_reader(BufReader::new(File::open("../../data/tests.json").unwrap()))
                .unwrap();

        let mut ok = true;
        for test in tests.iter().filter(|test| {
            !test
                .output
                .iter()
                .any(|lane| matches!(lane, Lane::Separator))
        }) {
            if !test.skip.is_none() && test.skip.unwrap() {
                continue;
            }
            let lc = Locale::builder().driving_side(test.driving_side).build();
            let input_road = Road {
                lanes: test.output.clone(),
            };
            let tags = lanes_to_tags(&test.output, &lc).unwrap();
            let output_road = get_lane_specs_ltr(&tags, &lc).unwrap();
            if input_road != output_road {
                ok = false;
                if !test.way_id.is_none() {
                    println!(
                        "For input (example from https://www.openstreetmap.org/way/{}):",
                        test.way_id.unwrap()
                    );
                } else if !test.link.is_none() {
                    println!("For input (example from {}):", test.link.as_ref().unwrap());
                }
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
            }
        }
        assert!(ok);
    }
}
