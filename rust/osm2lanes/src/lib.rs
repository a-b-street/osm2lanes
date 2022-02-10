//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

// Use `wee_alloc` as the global allocator.
#[cfg(feature = "wasm")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

mod metre;
pub use metre::Metre;

pub mod road;

pub mod tag;

mod locale;
pub use self::locale::{DrivingSide, Locale};

pub mod transform;
pub use self::transform::{lanes_to_tags, tags_to_lanes, LanesToTagsConfig, TagsToLanesConfig};

#[cfg(feature = "overpass")]
pub mod overpass;

#[cfg(feature = "wasm")]
mod wasm;

mod test;
