use osm_tags::TagKey;

use crate::locale::DrivingSide;
use crate::road::{Designated, Direction, Lane};

mod error;
pub use error::{RoadError, RoadFromTags, RoadWarnings};

mod tags_to_lanes;
pub use tags_to_lanes::{tags_to_lanes, Config as TagsToLanesConfig, Infer, TagsToLanesMsg};

mod lanes_to_tags;
pub use lanes_to_tags::{lanes_to_tags, Config as LanesToTagsConfig, LanesToTagsMsg};

pub mod tags {
    use osm_tags::TagKey;

    pub const CYCLEWAY: TagKey = TagKey::from_static("cycleway");
    pub const SIDEWALK: TagKey = TagKey::from_static("sidewalk");
    pub const SHOULDER: TagKey = TagKey::from_static("shoulder");
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

impl From<DrivingSide> for WaySide {
    fn from(side: DrivingSide) -> Self {
        match side {
            DrivingSide::Right => Self::Right,
            DrivingSide::Left => Self::Left,
        }
    }
}

impl From<DrivingSide> for TagKey {
    fn from(side: DrivingSide) -> Self {
        match side {
            DrivingSide::Right => Self::from("right"),
            DrivingSide::Left => Self::from("left"),
        }
    }
}

impl DrivingSide {
    fn tag(self) -> TagKey {
        self.into()
    }
}

impl Lane {
    #[must_use]
    pub fn is_separator(&self) -> bool {
        matches!(self, Lane::Separator { .. })
    }

    #[must_use]
    fn is_motor(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: Designated::Motor,
                ..
            }
        )
    }

    #[must_use]
    pub fn is_foot(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: Designated::Foot,
                ..
            }
        )
    }

    #[must_use]
    fn is_bicycle(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: Designated::Bicycle,
                ..
            }
        )
    }

    #[must_use]
    fn is_bus(&self) -> bool {
        matches!(
            self,
            Lane::Travel {
                designated: Designated::Bus,
                ..
            }
        )
    }

    #[must_use]
    fn direction(&self) -> Option<Direction> {
        match self {
            Self::Travel { direction, .. } => *direction,
            _ => None,
        }
    }
}
