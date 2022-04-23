use crate::locale::{DrivingSide, Locale};
use crate::road::{Designated, Direction, Lane};
use crate::tag::TagKey;

mod error;
pub use error::{RoadError, RoadFromTags, RoadWarnings};

pub mod tags_to_lanes;
pub use tags_to_lanes::{tags_to_lanes, Config as TagsToLanesConfig, TagsToLanesMsg};

mod lanes_to_tags;
pub use lanes_to_tags::{lanes_to_tags, Config as LanesToTagsConfig, LanesToTagsMsg};

pub mod tags {
    use crate::tag::TagKey;

    pub const CYCLEWAY: TagKey = TagKey::from("cycleway");
    pub const SIDEWALK: TagKey = TagKey::from("sidewalk");
    pub const SHOULDER: TagKey = TagKey::from("shoulder");
}

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
    fn tag(self) -> TagKey {
        self.into()
    }
}

impl Lane {
    #[must_use]
    pub fn is_separator(&self) -> bool {
        matches!(self, Lane::Separator { .. })
    }

    fn forward(designated: Designated, locale: &Locale) -> Self {
        Self::Travel {
            direction: Some(Direction::Forward),
            designated,
            width: Some(locale.travel_width(&designated)),
            max_speed: None,
            access: None,
        }
    }

    fn backward(designated: Designated, locale: &Locale) -> Self {
        Self::Travel {
            direction: Some(Direction::Backward),
            designated,
            width: Some(locale.travel_width(&designated)),
            max_speed: None,
            access: None,
        }
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
