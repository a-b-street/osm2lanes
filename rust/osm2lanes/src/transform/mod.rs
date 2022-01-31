use serde::{Deserialize, Serialize};

use crate::road::{Lane, LaneDesignated, LaneDirection};
use crate::tags::{DuplicateKeyError, TagKey, Tags};
use crate::DrivingSide;

mod tags_to_lanes;
pub use tags_to_lanes::tags_to_lanes;
pub use tags_to_lanes::TagsToLanesConfig;

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
    fn set_bus(&mut self) -> ModeResult {
        match self {
            Self::Travel { designated, .. } => *designated = LaneDesignated::Bus,
            _ => unreachable!(),
        }
        Ok(())
    }
    fn direction(&self) -> Option<LaneDirection> {
        match self {
            Self::Travel { direction, .. } => *direction,
            _ => None,
        }
    }
}

// Errors

/// Tranformation Logic Issue
///
/// ```
/// use osm2lanes::transform::RoadMsg;
/// let _ = RoadMsg::deprecated_tag("foo", "bar");
/// let _ = RoadMsg::unsupported_tag("foo", "bar");
/// let _ = RoadMsg::unsupported_str("foo=bar because x and y");
/// ```
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoadMsg {
    // Deprecated OSM tags, with suggested alternative
    Deprecated {
        deprecated_tags: Tags,
        suggested_tags: Option<Tags>,
    },
    // Tag combination that is unsupported, and may never be supported
    Unsupported {
        description: Option<String>,
        tags: Option<Tags>,
    },
    // Tag combination that is known, but has yet to be implemented
    Unimplemented {
        description: Option<String>,
        tags: Option<Tags>,
    },
    // Tag combination that is ambiguous, and may never be supported
    Ambiguous {
        description: Option<String>,
        tags: Option<Tags>,
    },
    // Other issue
    Other {
        description: String,
        tags: Tags,
    },
    // Internal Errors
    TagsDuplicateKey(DuplicateKeyError),
}

impl RoadMsg {
    pub fn deprecated_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        Self::Unsupported {
            description: None,
            tags: Some(Tags::from_str_pairs(&[[key.into().as_str(), val]]).unwrap()),
        }
    }
    pub fn unsupported_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        Self::Unsupported {
            description: None,
            tags: Some(Tags::from_str_pairs(&[[key.into().as_str(), val]]).unwrap()),
        }
    }
    pub fn unimplemented_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        Self::Unimplemented {
            description: None,
            tags: Some(Tags::from_str_pairs(&[[key.into().as_str(), val]]).unwrap()),
        }
    }
    pub fn unsupported_str(description: &str) -> Self {
        Self::Unsupported {
            description: Some(description.to_owned()),
            tags: None,
        }
    }
}

impl std::fmt::Display for RoadMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Deprecated {
                deprecated_tags, ..
            } => write!(
                f,
                "deprecated: {}",
                deprecated_tags.to_vec().as_slice().join(" ")
            ),
            Self::Unsupported { description, tags }
            | Self::Unimplemented { description, tags }
            | Self::Ambiguous { description, tags } => {
                let tags = tags.as_ref().map(|tags| tags.to_vec().as_slice().join(" "));
                let prefix = match self {
                    Self::Unsupported { .. } => "unsupported",
                    Self::Unimplemented { .. } => "unimplemented",
                    Self::Ambiguous { .. } => "ambiguous",
                    _ => unreachable!(),
                };
                match (description, tags) {
                    (None, None) => write!(f, "{}", prefix),
                    (Some(description), None) => {
                        write!(f, "{}: {}", prefix, description)
                    }
                    (None, Some(tags)) => write!(f, "{}: {}", prefix, tags),
                    (Some(description), Some(tags)) => {
                        write!(f, "{}: {}, {}", prefix, description, tags)
                    }
                }
            }
            Self::Other { description, .. } => write!(f, "{}", description),
            Self::TagsDuplicateKey(e) => e.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RoadWarnings(Vec<RoadMsg>);

impl RoadWarnings {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn push(&mut self, msg: RoadMsg) {
        self.0.push(msg)
    }
}

impl std::fmt::Display for RoadWarnings {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|warning| format!("Warning: {}", warning))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoadError {
    Msg(RoadMsg),
    Warnings(RoadWarnings),
    RoundTrip,
}

impl RoadError {
    fn ambiguous_str(description: &str) -> Self {
        RoadMsg::unsupported_str(description).into()
    }
}

impl std::error::Error for RoadError {}

impl std::fmt::Display for RoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Msg(msg) => msg.fmt(f),
            Self::Warnings(warnings) => write!(f, "{} warnings", warnings.0.len()),
            Self::RoundTrip => write!(f, "lanes to tags cannot roundtrip"),
        }
    }
}

impl From<RoadMsg> for RoadError {
    fn from(msg: RoadMsg) -> Self {
        Self::Msg(msg)
    }
}

impl From<RoadWarnings> for RoadError {
    fn from(warnings: RoadWarnings) -> Self {
        Self::Warnings(warnings)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lanes {
    pub lanes: Vec<Lane>,
    pub warnings: RoadWarnings,
}

type ModeResult = Result<(), RoadError>;
type LanesResult = Result<Lanes, RoadError>;
type TagsResult = Result<Tags, RoadError>;
