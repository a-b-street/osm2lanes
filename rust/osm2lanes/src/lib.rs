//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

mod metre;
pub use metre::Metre;

pub mod road;

pub mod tags;

mod locale;
pub use self::locale::{DrivingSide, Locale};

pub mod transform;
pub use self::transform::{lanes_to_tags, tags_to_lanes, LanesToTagsConfig, TagsToLanesConfig};
