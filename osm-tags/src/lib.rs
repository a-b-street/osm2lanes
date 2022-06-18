//! OSM Tags
//!
//! Provides `Tags`, `TagKey`, and `TagVal` structures to represent and help manipulate OpenStreetMap tags

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
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::str::FromStr;

mod key;
pub use key::TagKey;

mod val;
pub use val::TagVal;

#[derive(Debug, Clone)]
pub struct DuplicateKeyError(TagKey);

impl From<String> for DuplicateKeyError {
    fn from(string: String) -> Self {
        DuplicateKeyError(TagKey::from(&string))
    }
}

impl std::fmt::Display for DuplicateKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "duplicate tag key {}", self.0.as_str())
    }
}

impl std::error::Error for DuplicateKeyError {}

/// A map from string keys to string values. This makes copies of strings for
/// convenience; don't use in performance sensitive contexts.
//
// BTreeMap chosen for deterministic serialization.
// We often need to compare output directly, so cannot tolerate reordering
//
// TODO: fix this in the serialization by having the keys sorted.
#[derive(Clone, Debug, Default)]
pub struct Tags {
    map: BTreeMap<TagKey, TagVal>,
}

impl Tags {
    /// Construct from iterator of pairs
    ///
    /// # Errors
    ///
    /// If a duplicate key is provided.
    ///
    pub fn from_pairs<I, K, V>(tags: I) -> Result<Self, DuplicateKeyError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<TagKey>,
        V: Into<TagVal>,
    {
        let mut map: BTreeMap<TagKey, TagVal> = BTreeMap::new();
        for tag_pair in tags {
            let key: TagKey = tag_pair.0.into();
            let val: TagVal = tag_pair.1.into();
            // This may become cleaner with https://github.com/rust-lang/rust/issues/82766
            match map.entry(key) {
                Entry::Vacant(entry) => entry.insert(val),
                Entry::Occupied(entry) => return Err(DuplicateKeyError(entry.remove_entry().0)),
            };
        }
        Ok(Self { map })
    }

    /// Construct from pair
    #[must_use]
    pub fn from_pair<K, V>(key: K, val: V) -> Self
    where
        K: Into<TagKey>,
        V: Into<TagVal>,
    {
        let mut map = BTreeMap::default();
        let duplicate_val = map.insert(key.into(), val.into());
        debug_assert!(duplicate_val.is_none());
        Self { map }
    }

    /// Expose data as vector of pairs
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Expose data as vector of pairs
    #[must_use]
    pub fn to_str_pairs(&self) -> Vec<(&str, &str)> {
        self.map
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// Vector of `=` separated strings
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        let pairs = self.to_str_pairs();
        pairs
            .into_iter()
            .map(|(key, val)| format!("{key}={val}"))
            .collect()
    }

    /// Get value from tags given a key
    pub fn get<Q>(&self, q: &Q) -> Option<&str>
    where
        TagKey: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        self.map.get(q).map(TagVal::as_str)
    }

    /// Return if tags key has value,
    /// return false if key does not exist.
    #[must_use]
    pub fn is<Q>(&self, q: &Q, v: &str) -> bool
    where
        TagKey: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        self.get(q) == Some(v)
    }

    /// Return if tags key has any of the values,
    /// return false if the key does not exist.
    #[must_use]
    pub fn is_any<Q>(&self, q: &Q, values: &[&str]) -> bool
    where
        TagKey: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        if let Some(v) = self.get(q) {
            values.contains(&v)
        } else {
            false
        }
    }

    /// Get a subset of the tags
    #[must_use]
    pub fn subset<'any, I, Q, O>(&self, keys: I) -> Self
    where
        I: IntoIterator<Item = &'any Q>,
        TagKey: Borrow<Q>,
        Q: 'any + Ord + Hash + Eq + ?Sized + ToOwned<Owned = O>,
        O: Into<TagKey>,
    {
        let mut map = Self::default();
        for key in keys {
            if let Some(val) = self.get(key) {
                let owned: O = key.to_owned();
                let insert = map.checked_insert(owned.into(), TagVal::from(val));
                debug_assert!(insert.is_ok());
            }
        }
        map
    }

    /// Get node given a key part
    pub fn pairs_with_stem<Q>(&self, q: &Q) -> Vec<(&str, &str)>
    where
        Q: AsRef<str> + ?Sized,
    {
        self.map
            .iter()
            .filter_map(|(key, val)| {
                key.as_str()
                    .starts_with(q.as_ref())
                    .then(|| (key.as_str(), val.as_str()))
            })
            .collect()
    }

    /// # Errors
    ///
    /// If duplicate key is inserted.   
    ///
    pub fn checked_insert<K: Into<TagKey>, V: Into<TagVal>>(
        &mut self,
        key: K,
        val: V,
    ) -> Result<(), DuplicateKeyError> {
        let key: TagKey = key.into();
        // This may become cleaner with https://github.com/rust-lang/rust/issues/82766
        match self.map.entry(key) {
            Entry::Vacant(entry) => entry.insert(val.into()),
            Entry::Occupied(entry) => return Err(DuplicateKeyError(entry.remove_entry().0)),
        };
        Ok(())
    }
}

#[derive(Debug)]
pub enum ParseTagsError {
    MissingEquals(String),
    DuplicateKey(DuplicateKeyError),
}

impl std::fmt::Display for ParseTagsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingEquals(_val) => write!(f, "tag must be = separated"),
            Self::DuplicateKey(duplicate_key_err) => duplicate_key_err.fmt(f),
        }
    }
}

impl std::error::Error for ParseTagsError {}

impl FromStr for Tags {
    type Err = ParseTagsError;

    /// Parse '=' separated tag pairs from a newline separated list.
    ///
    /// ```
    /// use std::str::FromStr;
    /// use osm_tags::Tags;
    /// let tags = Tags::from_str("foo=bar\nabra=cadabra").unwrap();
    /// assert_eq!(tags.get("foo"), Some("bar"));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tags = s
            .lines()
            .map(|line| {
                let (key, val) = line
                    .split_once('=')
                    .ok_or_else(|| ParseTagsError::MissingEquals(line.to_owned()))?;
                Ok((key.to_owned(), val.to_owned()))
            })
            .collect::<Result<Vec<(String, String)>, Self::Err>>()?;
        Self::from_pairs(tags).map_err(ParseTagsError::DuplicateKey)
    }
}

impl ToString for Tags {
    /// Return tags as an '=' separated list
    ///
    /// ```
    /// use std::str::FromStr;
    /// use std::string::ToString;
    /// use osm_tags::Tags;
    /// let tags = Tags::from_str("foo=bar\nabra=cadabra").unwrap();
    /// assert_eq!(tags.to_string(), "abra=cadabra\nfoo=bar");
    /// ```
    fn to_string(&self) -> String {
        self.to_vec().as_slice().join("\n")
    }
}

/// A Visitor holds methods that a Deserializer can drive
#[cfg(feature = "serde")]
struct TagsVisitor {
    marker: std::marker::PhantomData<fn() -> Tags>,
}

#[cfg(feature = "serde")]
impl TagsVisitor {
    fn new() -> Self {
        TagsVisitor {
            marker: std::marker::PhantomData,
        }
    }
}

/// Visitor to Deserialize of Tags
#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for TagsVisitor {
    type Value = Tags;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("OSM Tags as Map")
    }
    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut tags = Tags::default();
        while let Some((key, value)) = access.next_entry::<String, String>()? {
            // Overpass sometimes returns duplicate tags
            let _ignored = tags.checked_insert(&key, value);
        }
        Ok(tags)
    }
}

/// Informs Serde how to deserialize Tags
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Tags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data, resulting in an instance of MyMap.
        deserializer.deserialize_map(TagsVisitor::new())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.map.len()))?;
        for (k, v) in &self.map {
            map.serialize_entry(k.as_str(), v.as_str())?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::{DuplicateKeyError, TagKey, Tags};

    #[test]
    fn test_tags() {
        let tags = Tags::from_pairs([
            ("foo", "bar"),
            ("abra", "cadabra"),
            ("foo:multi:key", "value"),
            ("multivalue", "apple;banana;chocolate covered capybara"),
        ])
        .unwrap();
        assert_eq!(
            tags.to_vec(),
            vec![
                "abra=cadabra",
                "foo=bar",
                "foo:multi:key=value",
                "multivalue=apple;banana;chocolate covered capybara"
            ]
        );

        // Serde
        let tags_str = "{\"abra\":\"cadabra\",\"foo\":\"bar\",\"foo:multi:key\":\"value\",\"multivalue\":\"apple;banana;chocolate covered capybara\"}";
        assert_eq!(serde_json::to_string(&tags).unwrap(), tags_str);
        let de_tags: Tags = serde_json::from_str(tags_str).unwrap();
        assert_eq!(de_tags.to_str_pairs(), tags.to_str_pairs());

        // Misc
        let mut other_tags = tags.clone();
        assert!(other_tags.checked_insert("new", "val").is_ok());
        assert!(matches!(
            other_tags.checked_insert("foo", "bar").unwrap_err(),
            DuplicateKeyError(_),
        ));
        assert!(other_tags
            .checked_insert(String::from("owned"), "val")
            .is_ok());

        // String interfaces
        assert_eq!(tags.get("foo"), Some("bar"));
        assert_eq!(tags.get("bar"), None);
        assert!(tags.is("foo", "bar"));
        assert!(!tags.is("foo", "foo"));
        assert!(!tags.is("bar", "foo"));
        assert!(tags.is_any("foo", &["bar"]));
        assert!(tags.is_any("foo", &["foo", "bar"]));
        assert!(!tags.is_any("foo", &["foo"]));
        assert!(!tags.is_any("bar", &["foo", "bar"]));
        assert_eq!(tags.subset(["foo"]).to_vec(), vec!["foo=bar"]);
        assert_eq!(
            tags.subset(["foo", "abra"]).to_vec(),
            vec!["abra=cadabra", "foo=bar"]
        );
        assert_eq!(tags.subset(["foo", "bar"]).to_vec(), vec!["foo=bar"]);
        assert!(tags.subset(["bar"]).to_vec().is_empty());

        // Key interfaces
        const FOO_KEY: TagKey = TagKey::from_static("foo");
        assert_eq!(tags.get(&FOO_KEY), Some("bar"));
        assert_eq!(tags.get(&TagKey::from_static("bar")), None);
        assert!(tags.is(&FOO_KEY, "bar"));
        assert!(!tags.is(&FOO_KEY, "foo"));
        assert_eq!(tags.subset(&[FOO_KEY]).to_vec(), vec!["foo=bar"]);
        dbg!(&(FOO_KEY + "multi" + "key"));
        assert!(tags.is(&(FOO_KEY + "multi" + "key"), "value"));
        let foo_key = FOO_KEY + "multi" + "key";
        assert!(tags.is(&foo_key, "value"));

        // Tree interfaces
        assert_eq!(tags.pairs_with_stem(&FOO_KEY).len(), 2);
        assert_eq!(tags.pairs_with_stem(&(FOO_KEY + "multi")).len(), 1);

        // TODO: Multi Value
    }
}
