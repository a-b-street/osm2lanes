use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

use crate::{keys, FromTagsDefault};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(IntoStaticStr, EnumString)]
#[strum(serialize_all = "snake_case")]
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
