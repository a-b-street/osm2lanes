use strum::{EnumString, IntoStaticStr};

use crate::{keys, FromTagsDefault};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Lit {
    Yes,
    No,
    SunsetSunrise,
    Automatic,
}

impl FromTagsDefault for Lit {
    const KEY: osm_tags::TagKey = keys::LIT;
}
