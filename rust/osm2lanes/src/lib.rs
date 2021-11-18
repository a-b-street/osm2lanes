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
    pub lane_type: LaneType,
    pub direction: Direction,
}

/// The type of a lane
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneType {
    /// A general-purpose lane for any vehicles
    Driving,
    /// On-street parking. May be diagonal, perpendicular, or parallel.
    Parking,
    /// A dedicated space for pedestrians, separated from the road by a curb.
    Sidewalk,
    /// Some roads without any sidewalks still have pedestrian traffic. This type represents the
    /// shoulder of the road, where people are usually forced to walk.
    Shoulder,
    /// A marked bike lane. May be separated from the rest of the road by some type of buffer.
    Biking,
    /// A bus-only lane
    Bus,
    /// A shared center turn lane
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
    Forward,
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
#[derive(Clone, Copy, PartialEq)]
pub enum DrivingSide {
    Right,
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

    #[test]
    fn test_osm_to_specs() {
        let mut ok = true;
        for (url, input, driving_side, expected_lt, expected_dir) in vec![
            (
                "https://www.openstreetmap.org/way/428294122",
                vec![
                    "lanes=2",
                    "oneway=yes",
                    "sidewalk=both",
                    "cycleway:left=lane",
                ],
                DrivingSide::Right,
                "sbdds",
                "v^^^^",
            ),
            (
                "https://www.openstreetmap.org/way/8591383",
                vec![
                    "lanes=1",
                    "oneway=yes",
                    "sidewalk=both",
                    "cycleway:left=track",
                    "oneway:bicycle=no",
                ],
                DrivingSide::Right,
                "sbbds",
                "vv^^^",
            ),
            (
                // A slight variation of the above, using cycleway:left:oneway=no, which should be
                // equivalent
                "https://www.openstreetmap.org/way/8591383",
                vec![
                    "lanes=1",
                    "oneway=yes",
                    "sidewalk=both",
                    "cycleway:left=track",
                    "cycleway:left:oneway=no",
                ],
                DrivingSide::Right,
                "sbbds",
                "vv^^^",
            ),
            (
                "https://www.openstreetmap.org/way/353690151",
                vec![
                    "lanes=4",
                    "sidewalk=both",
                    "parking:lane:both=parallel",
                    "cycleway:right=track",
                    "cycleway:right:oneway=no",
                ],
                DrivingSide::Right,
                "spddddbbps",
                "vvvv^^v^^^",
            ),
            (
                "https://www.openstreetmap.org/way/389654080",
                vec![
                    "lanes=2",
                    "sidewalk=both",
                    "parking:lane:left=parallel",
                    "parking:lane:right=no_stopping",
                    "centre_turn_lane=yes",
                    "cycleway:right=track",
                    "cycleway:right:oneway=no",
                ],
                DrivingSide::Right,
                "spdCdbbs",
                "vvv^^v^^",
            ),
            (
                "https://www.openstreetmap.org/way/369623526",
                vec![
                    "lanes=1",
                    "oneway=yes",
                    "sidewalk=both",
                    "parking:lane:right=diagonal",
                    "cycleway:left=opposite_track",
                    "oneway:bicycle=no",
                ],
                DrivingSide::Right,
                "sbbdps",
                "vv^^^^",
            ),
            (
                "https://www.openstreetmap.org/way/534549104",
                vec![
                    "lanes=2",
                    "oneway=yes",
                    "sidewalk=both",
                    "cycleway:right=track",
                    "cycleway:right:oneway=no",
                    "oneway:bicycle=no",
                ],
                DrivingSide::Right,
                "sddbbs",
                "v^^v^^",
            ),
            (
                "https://www.openstreetmap.org/way/777565028",
                vec!["highway=residential", "oneway=no", "sidewalk=both"],
                DrivingSide::Left,
                "sdds",
                "^^vv",
            ),
            (
                "https://www.openstreetmap.org/way/224637155",
                vec!["lanes=2", "oneway=yes", "sidewalk=left"],
                DrivingSide::Left,
                "sdd",
                "^^^",
            ),
            (
                "https://www.openstreetmap.org/way/4188078",
                vec![
                    "lanes=2",
                    "cycleway:left=lane",
                    "oneway=yes",
                    "sidewalk=left",
                ],
                DrivingSide::Left,
                "sbdd",
                "^^^^",
            ),
            (
                "https://www.openstreetmap.org/way/49207928",
                vec!["cycleway:right=lane", "sidewalk=both"],
                DrivingSide::Left,
                "sddbs",
                "^^vvv",
            ),
            // How should an odd number of lanes forward/backwards be split without any clues?
            (
                "https://www.openstreetmap.org/way/898731283",
                vec!["lanes=3", "sidewalk=both"],
                DrivingSide::Left,
                "sddds",
                "^^^vv",
            ),
            (
                // I didn't look for a real example of this
                "https://www.openstreetmap.org/way/898731283",
                vec!["lanes=5", "sidewalk=none"],
                DrivingSide::Right,
                "SdddddS",
                "vvv^^^^",
            ),
            (
                "https://www.openstreetmap.org/way/335668924",
                vec!["lanes=1", "sidewalk=none"],
                DrivingSide::Right,
                "SddS",
                "vv^^",
            ),
        ] {
            let cfg = Config {
                driving_side,
                inferred_sidewalks: true,
            };
            let actual = get_lane_specs_ltr(tags(input.clone()), &cfg);
            let actual_lt: String = actual.iter().map(|s| s.lane_type.to_char()).collect();
            let actual_dir: String = actual
                .iter()
                .map(|s| {
                    if s.direction == Direction::Forward {
                        '^'
                    } else {
                        'v'
                    }
                })
                .collect();
            if actual_lt != expected_lt || actual_dir != expected_dir {
                ok = false;
                println!("For input (example from {}):", url);
                for kv in input {
                    println!("    {}", kv);
                }
                println!("Got:");
                println!("    {}", actual_lt);
                println!("    {}", actual_dir);
                println!("Expected:");
                println!("    {}", expected_lt);
                println!("    {}", expected_dir);
                println!();
            }
        }
        assert!(ok);
    }

    fn tags(kv: Vec<&str>) -> BTreeMap<String, String> {
        let mut tags = BTreeMap::new();
        for pair in kv {
            let parts = pair.split('=').collect::<Vec<_>>();
            tags.insert(parts[0].to_string(), parts[1].to_string());
        }
        tags
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

        /// The inverse of `to_char`. Always picks one buffer type. Panics on invalid input.
        pub fn from_char(x: char) -> LaneType {
            match x {
                'd' => LaneType::Driving,
                'b' => LaneType::Biking,
                'B' => LaneType::Bus,
                'p' => LaneType::Parking,
                's' => LaneType::Sidewalk,
                'S' => LaneType::Shoulder,
                'C' => LaneType::SharedLeftTurn,
                'x' => LaneType::Construction,
                '|' => LaneType::Buffer(BufferType::FlexPosts),
                _ => panic!("from_char({}) undefined", x),
            }
        }
    }
}
