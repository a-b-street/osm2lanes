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
#![warn(unused_crate_dependencies)]
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
use std::collections::BTreeMap;
use std::str::FromStr;

mod key;
pub use key::{TagKey, TagKeyPart};

// mod osm;
// pub use osm::{Highway, HighwayImportance, HighwayType, Lifecycle, HIGHWAY, LIFECYCLE, ONEWAY};

mod access;
pub use access::Access;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct DuplicateKeyError(TagKey);

impl From<String> for DuplicateKeyError {
    fn from(string: String) -> Self {
        DuplicateKeyError(TagKey::from(string))
    }
}

impl std::fmt::Display for DuplicateKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "duplicate tag key {}", self.0.to_string())
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
    tree: TagTree,
}

impl Tags {
    /// Construct from slice of pairs
    ///
    /// # Errors
    ///
    /// If a duplicate key is provided.
    ///
    pub fn from_string_pairs<I: IntoIterator<Item = [String; 2]>>(
        tags: I,
    ) -> Result<Self, DuplicateKeyError> {
        let mut tree = TagTree::default();
        for tag_pair in tags {
            let [key, val] = dbg!(tag_pair);
            tree.insert(&dbg!(TagKey::from_string(key)).0, val)?;
        }
        Ok(dbg!(Self { tree }))
    }

    /// Construct from slice of pairs
    ///
    /// # Errors
    ///
    /// If a duplicate key is provided.
    ///
    pub fn from_str_pairs(tags: &[[&str; 2]]) -> Result<Self, DuplicateKeyError> {
        Self::from_string_pairs(
            tags.iter()
                .map(|pair| [pair[0].to_owned(), pair[1].to_owned()])
                .collect::<Vec<_>>(),
        )
    }

    /// Construct from pair
    #[must_use]
    pub fn from_string_pair(tag_pair: [String; 2]) -> Self {
        let [key, val] = tag_pair;
        let mut tree = TagTree::default();
        tree.insert(&TagKey::from_string(key).0, val).unwrap();
        Self { tree }
    }

    /// Construct from pair
    #[must_use]
    pub fn from_str_pair(tag: [&str; 2]) -> Self {
        Self::from_string_pair([tag[0].to_owned(), tag[1].to_owned()])
    }

    /// Expose data as vector of pairs
    #[must_use]
    pub fn to_str_pairs(&self) -> Vec<(String, &str)> {
        self.tree.to_str_pairs()
    }

    /// Vector of `=` separated strings
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        let pairs = self.tree.to_str_pairs();
        pairs
            .into_iter()
            .map(|(mut key, val)| {
                key.push('=');
                key.push_str(val);
                key
            })
            .collect()
    }

    /// Get value from tags given a key
    pub fn get(&self, q: &'static str) -> Option<&str> {
        self.tree
            .get(&TagKey::from_ref(q).0)
            .and_then(TagTreeVal::val)
    }
    // pub fn get<Q, B>(&self, q: &Q) -> Option<&str>
    // where
    //     Q: Into<B> + std::hash::Hash + Eq + Ord + ?Sized,
    //     TagKeyPart: Borrow<B> + Ord,
    // {
    //     todo!();
    //     // self.tree.get()
    // }

    /// Return if tags key has value,
    /// return false if key does not exist.
    #[must_use]
    pub fn is(&self, q: &'static str, v: &str) -> bool {
        self.get(q) == Some(v)
    }

    /// Return if tags key has any of the values,
    /// return false if the key does not exist.
    #[must_use]
    pub fn is_any<T: AsRef<str>>(&self, k: T, values: &[&str]) -> bool {
        todo!();
        // if let Some(v) = self.get(k) {
        //     values.contains(&v)
        // } else {
        //     false
        // }
    }

    /// Get a subset of the tags
    // TODO, find a way to do this without so many clones
    #[must_use]
    pub fn subset<T>(&self, keys: &[T]) -> Self
    where
        T: Clone + AsRef<str>,
    {
        todo!();
        // let mut map = Self::default();
        // for key in keys {
        //     if let Some(val) = self.get(key) {
        //         debug_assert!(map
        //             .checked_insert(key.as_ref().to_owned(), val.to_owned())
        //             .is_ok());
        //     }
        // }
        // map
    }

    #[must_use]
    pub fn tree(&self) -> &TagTree {
        &self.tree
    }

    /// # Errors
    ///
    /// If duplicate key is inserted.   
    ///
    pub fn checked_insert<K: Into<TagKey>, V: Into<String>>(
        &mut self,
        k: K,
        v: V,
    ) -> Result<(), DuplicateKeyError> {
        todo!();
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
                Ok([key, val])
            })
            .collect::<Result<Vec<_>, Self::Err>>()?;
        Self::from_str_pairs(&tags).map_err(ParseTagsError::DuplicateKey)
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
struct TagsVisitor {
    marker: std::marker::PhantomData<fn() -> Tags>,
}

impl TagsVisitor {
    fn new() -> Self {
        TagsVisitor {
            marker: std::marker::PhantomData,
        }
    }
}

/// Trait for Deserializers of Tags.
impl<'de> serde::de::Visitor<'de> for TagsVisitor {
    type Value = Tags;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("OSM Tags")
    }
    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut tags = Tags::default();
        // For when this becomes important:
        //let mut map = Tags::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry::<String, String>()? {
            // TODO
            tags.checked_insert(key, value).unwrap();
        }

        Ok(tags)
    }
}

/// Informs Serde how to deserialize Tags.
impl<'de> Deserialize<'de> for Tags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data, resulting in an instance of MyMap.
        deserializer.deserialize_map(TagsVisitor::new())
    }
}

#[derive(Clone, Default, Debug)]
pub struct TagTree(BTreeMap<TagKeyPart, TagTreeVal>);

impl TagTree {
    fn insert(&mut self, parts: &[TagKeyPart], val: String) -> Result<(), DuplicateKeyError> {
        if let Some(key) = parts.get(0) {
            self.0
                .entry(key.to_owned())
                .or_default()
                .insert(parts.get(1..), val)
        } else {
            // TODO: enforce this at compile time using a non-empty slice?
            unreachable!()
        }
    }

    /// Get tree node
    pub fn get(&self, parts: &[TagKeyPart]) -> Option<&TagTreeVal> {
        let next = self.0.get(parts.get(0).unwrap());
        if let Some(rest_key) = parts.get(1..) {
            if !rest_key.is_empty() {
                return next?.tree.as_ref()?.get(rest_key);
            }
        }
        next
    }

    /// Expose data as vector of pairs
    #[must_use]
    pub fn to_str_pairs(&self) -> Vec<(String, &str)> {
        self.0
            .iter()
            .map(|(parent_key, v)| {
                let mut ret_pairs = Vec::new();
                if let Some(nested_val) = v.val() {
                    ret_pairs.push((parent_key.to_string(), nested_val))
                }
                if let Some(nested_pairs) = v.tree.as_ref().map(|tree| tree.to_str_pairs()) {
                    for nested_pair in nested_pairs {
                        let (part_key, tag_val) = nested_pair;
                        ret_pairs.push((
                            format!("{}:{}", parent_key.as_str(), part_key.as_str()),
                            tag_val,
                        ))
                    }
                }
                ret_pairs
            })
            .flatten()
            .collect::<Vec<_>>()
    }
}

#[derive(Clone, Default, Debug)]
pub struct TagTreeVal {
    tree: Option<TagTree>,
    val: Option<String>,
}

impl TagTreeVal {
    fn insert(
        &mut self,
        parts: Option<&[TagKeyPart]>,
        val: String,
    ) -> Result<(), DuplicateKeyError> {
        match parts {
            Some(parts) if parts.len() > 0 => self
                .tree
                .get_or_insert_with(TagTree::default)
                .insert(parts, val),
            Some(_) | None => self.set(val),
        }
    }

    fn set(&mut self, input: String) -> Result<(), DuplicateKeyError> {
        if let Some(val) = &self.val {
            return Err(val.clone().into());
        }
        self.val = Some(input);
        Ok(())
    }

    /// Get nested value from tree given key
    #[must_use]
    pub fn get(&self, parts: &[TagKeyPart]) -> Option<&TagTreeVal> {
        self.tree.as_ref()?.get(parts)
    }

    /// Get value of root
    #[must_use]
    pub fn val(&self) -> Option<&str> {
        Some(self.val.as_ref()?.as_str())
    }

    /// Get tree
    #[must_use]
    pub fn tree(&self) -> Option<&TagTree> {
        self.tree.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::{TagKey, TagKeyPart, Tags};

    #[test]
    fn test_tags() {
        let tags = Tags::from_str_pairs(&[
            ["foo", "bar"],
            ["abra", "cadabra"],
            ["foo:multi:key", "value"],
            ["multivalue", "apple;banana;chocolate covered capybara"],
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

        // String interfaces
        assert_eq!(tags.get("foo"), Some("bar"));
        assert_eq!(tags.get("bar"), None);
        assert!(tags.is("foo", "bar"));
        assert!(!tags.is("foo", "foo"));
        assert!(!tags.is("bar", "foo"));
        // assert!(tags.is_any("foo", &["bar"]));
        // assert!(tags.is_any("foo", &["foo", "bar"]));
        // assert!(!tags.is_any("foo", &["foo"]));
        // assert!(!tags.is_any("bar", &["foo", "bar"]));
        // assert_eq!(tags.subset(&["foo"]).to_vec(), vec!["foo=bar"]);
        // assert_eq!(
        //     tags.subset(&["foo", "abra"]).to_vec(),
        //     vec!["abra=cadabra", "foo=bar"]
        // );
        // assert_eq!(tags.subset(&["foo", "bar"]).to_vec(), vec!["foo=bar"]);
        // assert!(tags.subset(&["bar"]).to_vec().is_empty());

        // Key interfaces
        // const FOO_KEY: TagKeyPart = TagKeyPart::from_static("foo");
        // assert!(tags.is(FOO_KEY, "bar"));
        // assert!(tags.is(&FOO_KEY, "bar"));
        // assert!(!tags.is(FOO_KEY, "foo"));
        // assert_eq!(tags.subset(&[FOO_KEY]).to_vec(), vec!["foo=bar"]);
        // assert!(tags.is(FOO_KEY + "multi" + "key", "value"));
        // let foo_key = FOO_KEY + "multi" + "key";
        // assert!(tags.is(&foo_key, "value"));

        // TODO: Tree interfaces

        // TODO: Multi Value
    }
}
