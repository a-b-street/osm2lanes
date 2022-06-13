use strum::{EnumString, IntoStaticStr};

use crate::{keys, FromTagsDefault};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TrackType {
    Grade1,
    Grade2,
    Grade3,
    Grade4,
    Grade5,
}

impl FromTagsDefault for TrackType {
    const KEY: osm_tags::TagKey = keys::TRACK_TYPE;
}
