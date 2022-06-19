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

use std::borrow::Borrow;
use std::hash::Hash;
use std::str::FromStr;

use osm_tags::{TagKey, Tags};

pub mod keys;

mod highway;
pub use highway::{Error as HighwayError, Highway, HighwayImportance, HighwayType, Lifecycle};

mod lit;
pub use lit::Lit;

mod track_type;
pub use track_type::TrackType;

mod smoothness;
pub use smoothness::Smoothness;

mod access;
pub use access::Access;

mod access_by_lane;
pub use access_by_lane::{Access as LaneAccess, LaneDependentAccess, LaneDependentAccessError};

#[derive(Debug)]
pub struct TagError<'tag>(TagKey, &'tag str);

impl std::fmt::Display for TagError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.0, self.1)
    }
}

impl std::error::Error for TagError<'_> {}

/// A single tagged key value pair
///
/// Either it is left untagged,
/// the tagged value is known,
/// or the tagged value is unknown.
pub enum Tagged<'tag, T> {
    /// Untagged
    None,
    Some(T),
    Unknown(&'tag str),
}

impl<'tag, T> Tagged<'tag, T> {
    /// Panics
    pub fn unwrap(self) -> T {
        match self {
            Tagged::None | Tagged::Unknown(_) => panic!(),
            Tagged::Some(v) => v,
        }
    }
    pub fn ok(self) -> Option<T> {
        match self {
            Tagged::None | Tagged::Unknown(_) => None,
            Tagged::Some(v) => Some(v),
        }
    }
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Tagged<'tag, U> {
        match self {
            Tagged::None => Tagged::None,
            Tagged::Some(t) => Tagged::Some(f(t)),
            Tagged::Unknown(s) => Tagged::Unknown(s),
        }
    }

    /// `Result<Option>`
    ///
    /// # Errors
    ///
    /// From `Tagged::Unknown`.
    pub fn ok_with(self, key: TagKey) -> Result<Option<T>, TagError<'tag>> {
        match self {
            Tagged::None => Ok(None),
            Tagged::Some(val) => Ok(Some(val)),
            Tagged::Unknown(s) => Err(TagError(key, s)),
        }
    }
}

trait FromTags: FromStr {
    /// From tags given key
    fn from_tags<'tag, Q>(tags: &'tag Tags, key: &Q) -> Tagged<'tag, Self>
    where
        TagKey: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        match tags.get(key) {
            Some(s) => match s.parse() {
                Ok(val) => Tagged::Some(val),
                Err(_) => Tagged::Unknown(s),
            },
            None => Tagged::None,
        }
    }
}

// Blanket impl
impl<T: FromStr> FromTags for T {}

trait FromTagsDefault: FromTags {
    const KEY: TagKey;

    /// From tags with default key
    fn from_tags_default(tags: &Tags) -> Tagged<Self> {
        Self::from_tags(tags, &Self::KEY)
    }
}

pub struct Schemes<'tag> {
    // Generic
    pub name: Option<String>,
    pub r#ref: Option<String>,

    // Ways
    pub highway: Result<Option<Highway>, HighwayError<'tag>>,
    pub lit: Result<Option<Lit>, TagError<'tag>>,
    pub tracktype: Result<Option<TrackType>, TagError<'tag>>,
    pub smoothness: Result<Option<Smoothness>, TagError<'tag>>,
}

impl<'tag> Schemes<'tag> {
    #[must_use]
    pub fn from_tags(tags: &'tag Tags) -> Self {
        Self {
            name: tags.get(&keys::NAME).map(ToOwned::to_owned),
            r#ref: tags.get(&keys::REF).map(ToOwned::to_owned),
            highway: Highway::from_tags(tags),
            lit: Lit::from_tags_default(tags).ok_with(Lit::KEY),
            tracktype: TrackType::from_tags_default(tags).ok_with(TrackType::KEY),
            smoothness: Smoothness::from_tags_default(tags).ok_with(Smoothness::KEY),
        }
    }
}
