use osm_tag_schemes::keys::ONEWAY;
use osm_tags::{TagKey, Tags};

use super::TagsToLanesMsg;
use crate::locale::Locale;
use crate::transform::RoadWarnings;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Oneway {
    Yes,
    No,
}

impl From<bool> for Oneway {
    fn from(oneway: bool) -> Self {
        if oneway {
            Oneway::Yes
        } else {
            Oneway::No
        }
    }
}

impl From<Oneway> for bool {
    fn from(oneway: Oneway) -> Self {
        match oneway {
            Oneway::Yes => true,
            Oneway::No => false,
        }
    }
}

impl Oneway {
    pub const KEY: TagKey = TagKey::from_static("oneway");

    pub fn from_tags(
        tags: &Tags,
        _locale: &Locale,
        _warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        Ok(
            match (tags.get(&ONEWAY), tags.is("junction", "roundabout")) {
                (Some("yes"), _) => Self::Yes,
                (Some("no"), false) => Self::No,
                (Some("no"), true) => {
                    return Err(TagsToLanesMsg::ambiguous_tags(
                        tags.subset(["oneway", "junction"]),
                    ));
                },
                (Some(value), _) => {
                    return Err(TagsToLanesMsg::unimplemented_tag(ONEWAY, value));
                },
                (None, roundabout) => Self::from(roundabout),
            },
        )
    }
}
