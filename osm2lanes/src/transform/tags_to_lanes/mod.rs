#![allow(clippy::module_name_repetitions)] // TODO: fix upstream

use crate::locale::Locale;
use crate::road::Road;
use crate::tag::Tags;
use crate::transform::error::{RoadError, RoadWarnings};
use crate::transform::RoadFromTags;

mod error;
pub use error::TagsToLanesMsg;

mod access_by_lane;
use access_by_lane::{Access, LaneDependentAccess};

mod counts;

mod modes;

mod separator;

mod road;
use road::{LaneBuilder, LaneBuilderError, LaneType, RoadBuilder};

mod unsupported;
use unsupported::unsupported;

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
    use crate::tag::{Tags, ONEWAY};
    use crate::transform::RoadWarnings;

    #[derive(Clone, Copy, PartialEq)]
    pub enum Oneway {
        Yes,
        No,
    }

    impl std::convert::From<bool> for Oneway {
        fn from(oneway: bool) -> Self {
            if oneway {
                Oneway::Yes
            } else {
                Oneway::No
            }
        }
    }

    impl std::convert::From<Oneway> for bool {
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
                match (tags.get(ONEWAY), tags.is("junction", "roundabout")) {
                    (Some("yes"), _) => Self::Yes,
                    (Some("no"), false) => Self::No,
                    (Some("no"), true) => {
                        return Err(TagsToLanesMsg::ambiguous_tags(
                            tags.subset(&["oneway", "junction"]),
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

mod infer;
pub use infer::Infer;

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

    // Create the road builder and start giving it schemes.
    let mut road: RoadBuilder = RoadBuilder::from(tags, locale, &mut warnings)?;

    modes::non_motorized(tags, locale, &mut road, &mut warnings)?;

    modes::bus(tags, locale, &mut road, &mut warnings)?;

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
