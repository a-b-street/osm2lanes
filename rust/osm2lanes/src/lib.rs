use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use self::transform::get_lane_specs_ltr;

mod transform;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaneSpec {
    pub lt: LaneType,
    pub dir: Direction,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneType {
    Driving,
    Parking,
    Sidewalk,
    // Walkable like a Sidewalk, but very narrow. Used to model pedestrians walking on roads
    // without sidewalks.
    Shoulder,
    Biking,
    Bus,
    SharedLeftTurn,
    Construction,
    LightRail,
    Buffer(BufferType),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum BufferType {
    /// Just paint!
    Stripes,
    /// Flex posts, wands, cones, other "weak" forms of protection. Can weave through them.
    FlexPosts,
    /// Sturdier planters, with gaps.
    Planters,
    /// Solid barrier, no gaps.
    JerseyBarrier,
    /// A raised curb
    Curb,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Forward,
    Backward,
}

pub struct Config {
    pub driving_side: DrivingSide,
    /// If true, roads without explicitly tagged sidewalks may have sidewalks or shoulders. If
    /// false, no sidewalks will be inferred if not tagged in OSM, and separate sidewalks will be
    /// included.
    pub inferred_sidewalks: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DrivingSide {
    Right,
    Left,
}

/// Convenience functions around a string->string map
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

impl LaneType {
    /// Represents the lane type as a single character, for use in tests.
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
            LaneType::LightRail => 'l',
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
            'l' => LaneType::LightRail,
            '|' => LaneType::Buffer(BufferType::FlexPosts),
            _ => panic!("from_char({}) undefined", x),
        }
    }
}
