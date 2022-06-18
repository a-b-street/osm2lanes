use osm_tag_schemes::LaneDependentAccess;
use osm_tags::{TagKey, Tags};

use crate::locale::Locale;
use crate::transform::{RoadWarnings, TagsToLanesMsg};

pub(in crate::transform::tags_to_lanes) struct Scheme(Option<LaneDependentAccess>);

impl Scheme {
    pub(crate) fn from_tags(
        tags: &Tags,
        _locale: &Locale,
        _warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        Ok(Self(LaneDependentAccess::from_tags(
            tags,
            &TagKey::from_static("cycleway:lanes"),
        )?))
    }
}
