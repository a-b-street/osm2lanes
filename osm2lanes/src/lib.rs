//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

#![warn(clippy::pedantic, clippy::cargo, clippy::restriction)]
// Allow cargo lints
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::multiple_crate_versions)] // itoa
// Allow restriction lints
#![allow(
    clippy::blanket_clippy_restriction_lints,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::expect_used,
    clippy::float_arithmetic,
    clippy::implicit_return,
    clippy::missing_docs_in_private_items,
    clippy::missing_inline_in_public_items,
    clippy::mod_module_files,
    clippy::multiple_inherent_impl,
    clippy::non_ascii_literal,
    clippy::pattern_type_mismatch,
    clippy::pub_use,
    clippy::same_name_method,
    clippy::separated_literal_suffix,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
    clippy::single_char_lifetime_names,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::wildcard_enum_match_arm
)]

pub mod locale;
pub mod metric;
pub mod road;
pub mod tag;

#[cfg(feature = "overpass")]
pub mod overpass;

pub mod transform;

#[cfg(feature = "tests")]
pub mod test;
