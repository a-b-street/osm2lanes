//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use self::transform::get_lane_specs_ltr;

mod transform;

/// A single lane
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaneSpec {
    #[serde(rename = "type")]
    pub lane_type: LaneType,
    pub direction: Direction,
}

/// The type of a lane
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneType {
    /// A general-purpose lane for any vehicles
    #[serde(rename = "travel_lane")]
    Driving,
    /// On-street parking. May be diagonal, perpendicular, or parallel.
    #[serde(rename = "parking_lane")]
    Parking,
    /// A dedicated space for pedestrians, separated from the road by a curb.
    #[serde(rename = "sidewalk")]
    Sidewalk,
    /// Some roads without any sidewalks still have pedestrian traffic. This type represents the
    /// shoulder of the road, where people are usually forced to walk.
    #[serde(rename = "shoulder")]
    Shoulder,
    /// A marked bike lane. May be separated from the rest of the road by some type of buffer.
    #[serde(rename = "cycleway")]
    Biking,
    /// A bus-only lane
    #[serde(rename = "bus_lane")]
    Bus,
    /// A shared center turn lane
    #[serde(rename = "shared_left_turn")]
    SharedLeftTurn,
    /// Some lane that's under construction
    Construction,
    Buffer(BufferType),
}

/// Some kind of physical or symbolic buffer, usually between motorized and non-motorized traffic.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum BufferType {
    /// Painted stripes
    Stripes,
    /// Flex posts, wands, cones, car ticklers, bollards, other "weak" forms of protection. Usually
    /// possible to weave through them.
    FlexPosts,
    /// Sturdier planters, with gaps
    Planters,
    /// Solid barrier, no gaps.
    JerseyBarrier,
    /// A raised curb
    Curb,
}

/// Is a lane oriented the same direction as the OSM way or not? See
/// https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right.
///
/// Note this concept needs to be thought through carefully. What direction does a parking lane
/// face? If there's bidirectional movement on a sidewalk, does a value make sense?
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    #[serde(rename = "forward")]
    Forward,
    #[serde(rename = "backward")]
    Backward,
}

/// Configuration to give extra context about the place where an OSM way exists.
pub struct Config {
    pub driving_side: DrivingSide,
    /// When sidewalks are not explicitly tagged on a way, should sidewalks or shoulder lanes be
    /// placed anyway based on heuristics?
    pub inferred_sidewalks: bool,
}

/// Do vehicles travel on the right or left side of a road?
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DrivingSide {
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "left")]
    Left,
}

/// Internal convenience functions around a string->string map
struct Tags(BTreeMap<String, String>);

impl Tags {
    pub fn new(map: BTreeMap<String, String>) -> Tags {
        Tags(map)
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.0.get(k)
    }

    pub fn is(&self, k: &str, v: &str) -> bool {
        self.0.get(k) == Some(&v.to_string())
    }

    pub fn is_any(&self, k: &str, values: Vec<&str>) -> bool {
        if let Some(v) = self.0.get(k) {
            values.contains(&v.as_ref())
        } else {
            false
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
        /// The OSM way unique identifier
        way_id: Option<i64>,
        link: Option<String>,
        tags: BTreeMap<String, String>,
        driving_side: DrivingSide,
        output: Vec<LaneSpec>,
        #[serde(rename = "skip_rust")]
        skip: Option<bool>,
    }

    #[test]
    fn test_from_data() {
        // TODO This is brittle, depends on running the test from the right directory. Use
        // include_str?
        let tests: Vec<TestCase> =
            serde_json::from_reader(BufReader::new(File::open("../../data/tests.json").unwrap()))
                .unwrap();

        let mut ok = true;
        for test in tests {
            if !test.skip.is_none() && test.skip.unwrap() {
                continue;
            }
            let cfg = Config {
                driving_side: test.driving_side,
                inferred_sidewalks: true,
            };
            let actual = get_lane_specs_ltr(test.tags.clone(), &cfg);
            if actual != test.output {
                ok = false;
                if !test.way_id.is_none() {
                    println!(
                        "For input (example from https://www.openstreetmap.org/way/{}):",
                        test.way_id.unwrap()
                    );
                } else if !test.link.is_none() {
                    println!("For input (example from {}):", test.link.unwrap());
                }
                for (k, v) in test.tags {
                    println!("    {} = {}", k, v);
                }
                println!("Got:");
                println!("    {}", stringify_lane_types(&actual));
                println!("    {}", stringify_directions(&actual));
                println!("Expected:");
                println!("    {}", stringify_lane_types(&test.output));
                println!("    {}", stringify_directions(&test.output));
                println!();
            }
        }
        assert!(ok);
    }

    fn stringify_lane_types(lanes: &[LaneSpec]) -> String {
        lanes.iter().map(|s| s.lane_type.to_char()).collect()
    }

    fn stringify_directions(lanes: &[LaneSpec]) -> String {
        lanes
            .iter()
            .map(|s| {
                if s.direction == Direction::Forward {
                    '^'
                } else {
                    'v'
                }
            })
            .collect()
    }

    impl LaneType {
        /// Represents the lane type as a single character. Always picks one buffer type.
        pub fn to_char(self) -> char {
            match self {
                LaneType::Driving => 'd',
                LaneType::Biking => 'b',
                LaneType::Bus => 'B',
                LaneType::Parking => 'p',
                LaneType::Sidewalk => 's',
                LaneType::Shoulder => 'S',
                LaneType::SharedLeftTurn => 'C',
                LaneType::Construction => 'x',
                LaneType::Buffer(_) => '|',
            }
        }
    }
}
