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
pub use highway::{Highway, HighwayImportance, HighwayType, Lifecycle};

mod track_type;
pub use track_type::TrackType;

mod smoothness;
pub use smoothness::Smoothness;

mod access;
pub use access::Access;

pub enum Tagged<'tag, T> {
    /// Untagged
    None,
    Some(T),
    Unknown(TagKey, &'tag str),
}

impl<'tag, T> Tagged<'tag, T> {
    /// Panics
    pub fn unwrap(self) -> T {
        match self {
            Tagged::None | Tagged::Unknown(_, _) => panic!(),
            Tagged::Some(v) => v,
        }
    }
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Tagged<'tag, U> {
        match self {
            Tagged::None => Tagged::None,
            Tagged::Some(t) => Tagged::Some(f(t)),
            Tagged::Unknown(k, v) => Tagged::Unknown(k, v),
        }
    }
    // TODO: panics
    pub fn or_insert(self, tags: &mut Tags) -> Option<T> {
        match self {
            Tagged::None => None,
            Tagged::Some(val) => Some(val),
            Tagged::Unknown(key, val) => {
                tags.checked_insert(key, val).unwrap();
                None
            },
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
                Err(_) => Tagged::Unknown(keys::TRACK_TYPE, s),
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

mod lit {
    use serde::{Deserialize, Serialize};
    use strum::{EnumString, IntoStaticStr};

    use crate::{keys, FromTagsDefault};

    #[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(IntoStaticStr, EnumString)]
    #[strum(serialize_all = "kebab-case")]
    pub enum Lit {
        Yes,
        No,
        SunsetSunrise,
        Automatic,
    }

    impl FromTagsDefault for Lit {
        const KEY: osm_tags::TagKey = keys::LIT;
    }
}
pub use lit::Lit;

pub struct Schemes {
    // Generic
    pub name: Option<String>,
    pub r#ref: Option<String>,

    // Ways
    pub highway: Option<Highway>,
    pub lit: Option<Lit>,
    pub tracktype: Option<TrackType>,
    pub smoothness: Option<Smoothness>,
}

impl Schemes {
    #[must_use]
    pub fn from_tags(tags: &Tags) -> (Self, Option<Tags>) {
        let mut unknown_tags = Tags::default();
        let schemes = Self {
            name: tags.get(&keys::NAME).map(std::borrow::ToOwned::to_owned),
            r#ref: tags.get(&keys::REF).map(ToOwned::to_owned),
            highway: Highway::from_tags(tags).or_insert(&mut unknown_tags),
            lit: Lit::from_tags_default(tags).or_insert(&mut unknown_tags),
            tracktype: TrackType::from_tags_default(tags).or_insert(&mut unknown_tags),
            smoothness: Smoothness::from_tags_default(tags).or_insert(&mut unknown_tags),
        };
        (
            schemes,
            if unknown_tags.is_empty() {
                None
            } else {
                Some(unknown_tags)
            },
        )
    }
}
