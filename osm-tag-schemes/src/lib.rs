//! OSM Tag Schemes
//!
//! Provides various structs and enums representing tagging schemes

#![warn(explicit_outlives_requirements)]
#![warn(missing_abi)]
#![deny(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(unreachable_pub)]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
// #![warn(unused_crate_dependencies)] // https://github.com/rust-lang/rust/issues/57274
#![warn(unused_lifetimes)]
#![warn(unused_qualifications)]
// Clippy
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cargo_common_metadata)]
#![warn(
    clippy::allow_attributes_without_reason,
    clippy::as_conversions,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::default_numeric_fallback,
    clippy::deref_by_slicing,
    clippy::empty_structs_with_brackets,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast_any,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::map_err_ignore,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::single_char_lifetime_names,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::todo,
    clippy::try_err,
    clippy::unseparated_literal_suffix,
    clippy::use_debug
)]

mod highway;
pub use highway::{Highway, HighwayImportance, HighwayType, Lifecycle};
use serde::{Deserialize, Serialize};

/// Access variants from <https://wiki.openstreetmap.org/wiki/Key:access#List_of_possible_values>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Yes,
    No,
    Private,
    Permissive,
    Permit,
    Destination,
    Delivery,
    Customers,
    Designated,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackType {
    Grade1,
    Grade2,
    Grade3,
    Grade4,
    Grade5,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Smoothness {
    Impassable,
    VeryHorrible,
    Horrible,
    VeryBad,
    Bad,
    Intermediate,
    Good,
    Excellent,
}
