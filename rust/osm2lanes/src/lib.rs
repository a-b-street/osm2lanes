//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]

pub mod locale;
pub mod metric;
pub mod road;
pub mod tag;

#[cfg(feature = "overpass")]
pub mod overpass;

pub mod transform;

#[cfg(feature = "tests")]
pub mod test;
