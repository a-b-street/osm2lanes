use crate::road::{Lane, LaneDesignated, LaneDirection};
use crate::tag::{TagKey, Tags};
use crate::DrivingSide;

mod error;
pub use error::{Lanes, RoadError, RoadMsg, RoadWarnings};

mod tags_to_lanes;
pub use tags_to_lanes::{tags_to_lanes, TagsToLanesConfig};

mod lanes_to_tags;
pub use lanes_to_tags::{lanes_to_tags, LanesToTagsConfig};

const HIGHWAY: TagKey = TagKey::from("highway");
const CYCLEWAY: TagKey = TagKey::from("cycleway");
const SIDEWALK: TagKey = TagKey::from("sidewalk");
const SHOULDER: TagKey = TagKey::from("shoulder");

pub type ModeResult = Result<(), RoadError>;
pub type LanesResult = Result<Lanes, RoadError>;
pub type TagsResult = Result<Tags, RoadError>;

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
    pub fn is_separator(&self) -> bool {
        matches!(self, Lane::Separator { .. })
    }
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
    pub fn is_foot(&self) -> bool {
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
    fn direction(&self) -> Option<LaneDirection> {
        match self {
            Self::Travel { direction, .. } => *direction,
            _ => None,
        }
    }
}
