use serde::{Deserialize, Serialize};

use crate::tags::{TagKey, Tags};
use crate::{DrivingSide, Lane, LaneDesignated, LaneDirection, Road, RoadError};

mod tags_to_lanes;
pub use tags_to_lanes::{tags_to_lanes, tags_to_lanes_with_warnings};

mod lanes_to_tags;
pub use lanes_to_tags::lanes_to_tags;
pub use lanes_to_tags::LanesToTagsConfig;

const HIGHWAY: TagKey = TagKey::from("highway");
const CYCLEWAY: TagKey = TagKey::from("cycleway");
const SIDEWALK: TagKey = TagKey::from("sidewalk");
const SHOULDER: TagKey = TagKey::from("shoulder");

#[derive(Clone, Debug, PartialEq)]
enum WaySide {
    Both,
    Right,
    Left,
}

impl WaySide {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Right => "right",
            Self::Left => "left",
        }
    }
}

impl ToString for WaySide {
    fn to_string(&self) -> String {
        self.as_str().to_owned()
    }
}

impl std::convert::From<DrivingSide> for WaySide {
    fn from(side: DrivingSide) -> Self {
        match side {
            DrivingSide::Right => Self::Right,
            DrivingSide::Left => Self::Left,
        }
    }
}

impl std::convert::From<DrivingSide> for TagKey {
    fn from(side: DrivingSide) -> Self {
        match side {
            DrivingSide::Right => Self::from("right"),
            DrivingSide::Left => Self::from("left"),
        }
    }
}

impl DrivingSide {
    fn tag(&self) -> TagKey {
        (*self).into()
    }
}

impl Lane {
    fn forward(designated: LaneDesignated) -> Self {
        Self::Travel {
            direction: Some(LaneDirection::Forward),
            designated,
        }
    }
    fn backward(designated: LaneDesignated) -> Self {
        Self::Travel {
            direction: Some(LaneDirection::Backward),
            designated,
        }
    }
    fn both(designated: LaneDesignated) -> Self {
        Self::Travel {
            direction: Some(LaneDirection::Both),
            designated,
        }
    }
    fn foot() -> Self {
        Self::Travel {
            direction: None,
            designated: LaneDesignated::Foot,
        }
    }
    fn parking(direction: LaneDirection) -> Self {
        Self::Parking {
            direction,
            designated: LaneDesignated::Motor,
        }
    }
    fn is_motor(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: LaneDesignated::Motor,
                ..
            }
        )
    }
    fn is_foot(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: LaneDesignated::Foot,
                ..
            }
        )
    }
    fn is_bicycle(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: LaneDesignated::Bicycle,
                ..
            }
        )
    }
    fn is_bus(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: LaneDesignated::Bus,
                ..
            }
        )
    }
    fn set_bus(&mut self) -> ModeResult {
        match self {
            Self::Travel { designated, .. } => *designated = LaneDesignated::Bus,
            _ => unreachable!(),
        }
        Ok(())
    }
    fn get_direction(&self) -> Option<LaneDirection> {
        match self {
            Self::Travel { direction, .. } => *direction,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneError(String);

impl ToString for LaneError {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LaneWarnings(Vec<LaneSpecWarning>);

impl LaneWarnings {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl ToString for LaneWarnings {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|warn| format!("Warning: {}", warn.description))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneSpecWarning {
    pub description: String,
    // Tags relevant to triggering the warning
    // TODO: investigate making this a view of keys on a Tags object instead
    pub tags: Tags,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lanes {
    pub lanes: Vec<Lane>,
    pub warnings: LaneWarnings,
}

impl From<String> for RoadError {
    fn from(s: String) -> Self {
        RoadError::Lane(LaneError(s))
    }
}

impl From<&'static str> for RoadError {
    fn from(s: &'static str) -> Self {
        RoadError::Lane(LaneError(s.to_owned()))
    }
}

type ModeResult = Result<(), RoadError>;
type LanesResult = Result<Lanes, RoadError>;
type RoadResult = Result<Road, RoadError>;
type TagsResult = Result<Tags, RoadError>;
