#![allow(clippy::module_name_repetitions)] // TODO: fix upstream

use std::borrow::Borrow;
use std::hash::Hash;

use crate::locale::Locale;
use crate::road::Road;
use crate::tag_keys::Tags;
use crate::transform::error::{RoadError, RoadWarnings};
use crate::transform::RoadFromTags;

mod error;
pub use error::TagsToLanesMsg;

mod access_by_lane;
use access_by_lane::{Access, LaneDependentAccess};

mod counts;

mod modes;
use modes::BuswayScheme;

mod separator;

mod road;
use osm_tags::TagKey;
use road::{LaneBuilder, LaneBuilderError, LaneType, RoadBuilder};

mod unsupported;
use unsupported::unsupported;

mod infer;
pub use infer::Infer;

trait TagsNumeric {
    fn get_parsed<Q, T, O>(&self, key: &Q, warnings: &mut RoadWarnings) -> Option<T>
    where
        TagKey: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized + ToOwned<Owned = O>,
        O: Into<TagKey>,
        T: std::str::FromStr;
}

impl TagsNumeric for Tags {
    fn get_parsed<Q, T, O>(&self, key: &Q, warnings: &mut RoadWarnings) -> Option<T>
    where
        TagKey: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized + ToOwned<Owned = O>,
        O: Into<TagKey>,
        T: std::str::FromStr,
    {
        self.get(key).and_then(|val| {
            if let Ok(w) = val.parse::<T>() {
                Some(w)
            } else {
                warnings.push(TagsToLanesMsg::unsupported_tag(key.to_owned(), val));
                None
            }
        })
    }
}

#[non_exhaustive]
pub struct Config {
    pub error_on_warnings: bool,
    pub include_separators: bool,
}

impl Config {
    #[must_use]
    pub fn new(error_on_warnings: bool, include_separators: bool) -> Self {
        Self {
            error_on_warnings,
            include_separators,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            error_on_warnings: false,
            include_separators: true,
        }
    }
}

mod oneway {
    use super::TagsToLanesMsg;
    use crate::locale::Locale;
    use crate::tag_keys::{Tags, ONEWAY};
    use crate::transform::RoadWarnings;

    #[derive(Clone, Copy, PartialEq)]
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
}
use oneway::Oneway;

pub(in crate::transform::tags_to_lanes) struct TagSchemes {
    oneway: Oneway,
    busway: BuswayScheme,
}

impl TagSchemes {
    pub(crate) fn from_tags(
        tags: &Tags,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        let oneway = Oneway::from_tags(tags, locale, warnings)?;
        let busway = BuswayScheme::from_tags(tags, oneway, locale, warnings)?;
        Ok(Self { oneway, busway })
    }
}

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
///
/// # Errors
///
/// Warnings or errors are produced for situations that may make the lanes inaccurate, such as:
///
/// - Unimplemented or unsupported tags
/// - Ambiguous tags
/// - Unknown internal errors
///
/// If the issue may be recoverable, a warning is preferred.
/// A config option allows all warnings to be treated as errors.
///
pub fn tags_to_lanes(
    tags: &Tags,
    locale: &Locale,
    config: &Config,
) -> Result<RoadFromTags, RoadError> {
    let mut warnings = RoadWarnings::default();

    // Early return if we find unimplemented or unsupported tags.
    unsupported(tags, locale, &mut warnings)?;

    // Parse each scheme independently ahead of time, to simplify the process and ensure local consistency
    let schemes = TagSchemes::from_tags(tags, locale, &mut warnings)?;

    // Create the road builder and start giving it schemes.
    let mut road: RoadBuilder = RoadBuilder::from(&schemes, tags, locale, &mut warnings)?;

    modes::non_motorized(tags, locale, &mut road, &mut warnings)?;

    modes::bus(&schemes.busway, tags, locale, &mut road, &mut warnings)?;

    modes::bicycle(tags, locale, &mut road, &mut warnings)?;

    modes::parking(tags, locale, &mut road)?;

    modes::foot_and_shoulder(tags, locale, &mut road, &mut warnings)?;

    let (lanes, highway, _oneway) =
        road.into_ltr(tags, locale, config.include_separators, &mut warnings)?;

    let road_from_tags = RoadFromTags {
        road: Road { lanes, highway },
        warnings,
    };

    if config.error_on_warnings && !road_from_tags.warnings.is_empty() {
        return Err(road_from_tags.warnings.into());
    }

    Ok(road_from_tags)
}
