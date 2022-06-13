use strum::{EnumString, IntoStaticStr};

use crate::{keys, FromTagsDefault};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Smoothness {
    Impassable,
    VeryHorrible,
    Horrible,
    VeryBad,
    Bad,
    Intermediate,
    Good,
    Excellent,
}

impl FromTagsDefault for Smoothness {
    const KEY: osm_tags::TagKey = keys::SMOOTHNESS;
}
