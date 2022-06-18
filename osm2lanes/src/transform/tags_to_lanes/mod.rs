#![allow(clippy::module_name_repetitions)] // TODO: fix upstream

use std::borrow::Borrow;
use std::hash::Hash;

use osm_tag_schemes::Schemes;
use osm_tags::Tags;

use crate::locale::Locale;
use crate::road::Road;
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

mod oneway;
use oneway::Oneway;

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

    let (generic_schemes, remainder_tags) = Schemes::from_tags(tags);
    if let Some(error_tags) = remainder_tags {
        warnings.push(TagsToLanesMsg::unsupported_tags(error_tags));
    }

    // Parse each scheme independently ahead of time, to simplify the process and ensure local consistency
    let crate_schemes = TagSchemes::from_tags(tags, locale, &mut warnings)?;

    // Create the road builder and start giving it schemes.
    let mut road: RoadBuilder = RoadBuilder::from(
        &generic_schemes,
        &crate_schemes,
        tags,
        locale,
        &mut warnings,
    )?;

    modes::non_motorized(tags, locale, &mut road, &mut warnings)?;

    modes::bus(
        &crate_schemes.busway,
        tags,
        locale,
        &mut road,
        &mut warnings,
    )?;

    modes::bicycle(tags, locale, &mut road, &mut warnings)?;

    modes::parking(tags, locale, &mut road)?;

    modes::foot_and_shoulder(tags, locale, &mut road, &mut warnings)?;

    let (lanes, highway, _oneway) =
        road.into_ltr(tags, locale, config.include_separators, &mut warnings)?;

    let road_from_tags = RoadFromTags {
        road: Road {
            name: generic_schemes.name,
            r#ref: generic_schemes.r#ref,
            highway,
            lit: generic_schemes.lit,
            tracktype: generic_schemes.tracktype,
            smoothness: generic_schemes.smoothness,
            lanes,
        },
        warnings,
    };

    if config.error_on_warnings && !road_from_tags.warnings.is_empty() {
        return Err(road_from_tags.warnings.into());
    }

    Ok(road_from_tags)
}
