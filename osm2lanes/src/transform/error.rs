use osm_tags::DuplicateKeyError;

use super::TagsToLanesMsg;
use crate::road::Road;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RoadWarnings(Vec<TagsToLanesMsg>);

impl RoadWarnings {
    #[must_use]
    pub fn new(msgs: Vec<TagsToLanesMsg>) -> Self {
        Self(msgs)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[TagsToLanesMsg] {
        self.0.as_slice()
    }

    pub fn push(&mut self, msg: TagsToLanesMsg) {
        self.0.push(msg);
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

/// Error for transformation
/// ```
/// use osm2lanes::transform::{TagsToLanesMsg, RoadError};
/// let msg: TagsToLanesMsg = TagsToLanesMsg::deprecated_tag("foo", "bar");
/// assert_eq!("\"deprecated: 'foo=bar' - src/transform/error.rs:5:27\"", serde_json::to_string(&msg).unwrap());
/// let err: RoadError = msg.into();
/// assert_eq!("{\"error\":\"deprecated: 'foo=bar' - src/transform/error.rs:5:27\"}", serde_json::to_string(&err).unwrap());
/// ```
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum RoadError {
    #[serde(rename = "error")]
    Msg(TagsToLanesMsg),
    #[serde(rename = "warnings")]
    Warnings(RoadWarnings),
    #[serde(rename = "round_trip")]
    RoundTrip,
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

impl From<DuplicateKeyError> for RoadError {
    #[track_caller]
    fn from(e: DuplicateKeyError) -> Self {
        e.into()
    }
}

impl From<TagsToLanesMsg> for RoadError {
    fn from(msg: TagsToLanesMsg) -> Self {
        Self::Msg(msg)
    }
}

impl From<RoadWarnings> for RoadError {
    fn from(warnings: RoadWarnings) -> Self {
        Self::Warnings(warnings)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RoadFromTags {
    pub road: Road,
    pub warnings: RoadWarnings,
}
