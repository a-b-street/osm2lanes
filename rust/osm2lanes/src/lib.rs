//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

mod metric;
pub use metric::{Metre, Speed};

pub mod road;

pub mod tag;

pub mod locale;
pub use locale::{DrivingSide, Locale};

#[cfg(feature = "overpass")]
pub mod overpass;

pub mod transform;
pub use transform::{lanes_to_tags, tags_to_lanes, LanesToTagsConfig, TagsToLanesConfig};

#[cfg(feature = "overpass")]
pub mod test;
