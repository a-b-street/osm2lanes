#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod key;
pub use key::TagKey;

mod osm;
pub use osm::{Highway, Lifecycle, HIGHWAY, LIFECYCLE};

use crate::transform::{RoadMsg, RoadWarnings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateKeyError(String);

impl std::fmt::Display for DuplicateKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "duplicate tag key {}", self.0)
    }
}

/// A map from string keys to string values. This makes copies of strings for
/// convenience; don't use in performance sensitive contexts.
//
// BTreeMap chosen for deterministic serialization.
// We often need to compare output directly, so cannot tolerate reordering
//
// TODO: fix this in the serialization by having the keys sorted.
//
// TODO: use only one of map or tree, with a zero-cost API for the other at runtime
//
#[derive(Clone, Debug, Default)]
pub struct Tags {
    map: BTreeMap<String, String>,
    tree: TagTree,
}

impl Tags {
    /// Construct from slice of pairs
    ///
    /// # Errors
    ///
    /// If a duplicate key is provided.
    ///
    pub fn from_str_pairs(tags: &[[&str; 2]]) -> Result<Self, DuplicateKeyError> {
        let mut map = BTreeMap::new();
        for tag in tags {
            map.insert(tag[0].to_owned(), tag[1].to_owned())
                .map_or(Ok(()), |_| Err(DuplicateKeyError(tag[0].to_owned())))?;
        }
        let mut tree = TagTree::default();
        for (k, v) in &map {
            tree.insert(k, v.clone())?;
        }
        Ok(Self { map, tree })
    }

    /// Construct from pair
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub(crate) fn from_str_pair(tag: [&str; 2]) -> Self {
        let mut map = BTreeMap::new();
        map.insert(tag[0].to_owned(), tag[1].to_owned());
        let mut tree = TagTree::default();
        tree.insert(tag[0], tag[1].to_owned()).unwrap();
        Self { map, tree }
    }

    /// Expose data as vector of pairs
    #[must_use]
    pub fn to_str_pairs(&self) -> Vec<[&str; 2]> {
        self.map
            .iter()
            .map(|(k, v)| [k.as_str(), v.as_str()])
            .collect::<Vec<_>>()
    }

    /// Vector of `=` separated strings
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        self.map
            .iter()
            .map(|(k, v)| format!("{}={}", k.as_str(), v.as_str()))
            .collect::<Vec<String>>()
    }

    /// Get value from tags given a key
    pub fn get<T: AsRef<str>>(&self, k: T) -> Option<&str> {
        self.map.get(k.as_ref()).map(String::as_str)
    }

    /// Get the value for the given key and parse it into T. Add a RoadMsg::Unsupported if parsing
    /// fails.
    pub fn get_parsed<K: AsRef<str>, T: FromStr>(
        &self,
        key: &K,
        warnings: &mut RoadWarnings,
    ) -> Option<T> {
        self.get(key).and_then(|val| match val.parse::<T>() {
            Ok(n) => Some(n),
            Err(_) => {
                warnings.push(RoadMsg::unsupported_tag(key.as_ref().to_owned(), val));
                None
            }
        })
    }

    /// Return if tags key has value,
    /// return false if key does not exist.
    #[must_use]
    pub fn is<T: AsRef<str>>(&self, k: T, v: &str) -> bool {
        self.get(k) == Some(v)
    }

    /// Return if tags key has any of the values,
    /// return false if the key does not exist.
    #[must_use]
    pub fn is_any<T: AsRef<str>>(&self, k: T, values: &[&str]) -> bool {
        if let Some(v) = self.get(k) {
            values.contains(&v)
        } else {
            false
        }
    }

    /// Get a subset of the tags
    // TODO, find a way to do this without so many clones
    #[must_use]
    pub fn subset<T>(&self, keys: &[T]) -> Self
    where
        T: Clone + AsRef<str>,
    {
        let mut map = Self::default();
        for key in keys {
            if let Some(val) = self.get(key) {
                debug_assert!(map
                    .checked_insert(key.as_ref().to_owned(), val.to_owned())
                    .is_ok());
            }
        }
        map
    }

    #[must_use]
    pub fn tree(&self) -> &TagTree {
        &self.tree
    }
}

impl FromStr for Tags {
    type Err = String;

    /// Parse tags from an '=' separated list
    ///
    /// ```
    /// use std::str::FromStr;
    /// use osm2lanes::tag::Tags;
    /// let tags = Tags::from_str("foo=bar\nabra=cadabra").unwrap();
    /// assert_eq!(tags.get("foo"), Some("bar"));
    /// ```
    #[allow(clippy::map_err_ignore)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tags = s
            .lines()
            .map(|line| {
                let (key, val) = line.split_once('=').ok_or("tag must be = separated")?;
                Ok([key, val])
            })
            .collect::<Result<Vec<_>, Self::Err>>()?;
        // TODO: better error handling
        Self::from_str_pairs(&tags).map_err(|_| "Duplicate Key Error".to_owned())
    }
}

impl ToString for Tags {
    /// Return tags as an '=' separated list
    ///
    /// ```
    /// use std::str::FromStr;
    /// use std::string::ToString;
    /// use osm2lanes::tag::Tags;
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

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Default, Debug)]
pub struct TagTree(BTreeMap<String, TagTreeVal>);

impl TagTree {
    fn insert(&mut self, key: &str, val: String) -> Result<(), DuplicateKeyError> {
        let (root_key, rest_key) = match key.split_once(':') {
            Some((left, right)) => (left, Some(right)),
            None => (key, None),
        };
        self.0
            .entry(root_key.to_owned())
            .or_default()
            .insert(rest_key, val)
    }
    pub fn get<K: Into<TagKey>>(&self, key: K) -> Option<&TagTreeVal> {
        let key: TagKey = key.into();
        let (root_key, rest_key) = match key.as_str().split_once(':') {
            Some((left, right)) => (left, Some(right)),
            None => (key.as_str(), None),
        };
        let next = self.0.get(root_key);
        if let Some(rest_key) = rest_key {
            next?
                .tree
                .as_ref()?
                .get::<TagKey>(rest_key.to_owned().into())
        } else {
            next
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Default, Debug)]
pub struct TagTreeVal {
    tree: Option<TagTree>,
    val: Option<String>,
}

impl TagTreeVal {
    fn insert(&mut self, key: Option<&str>, val: String) -> Result<(), DuplicateKeyError> {
        match key {
            Some(key) => self
                .tree
                .get_or_insert_with(TagTree::default)
                .insert(key, val),
            None => self.set(val),
        }
    }

    fn set(&mut self, input: String) -> Result<(), DuplicateKeyError> {
        if let Some(val) = &self.val {
            return Err(DuplicateKeyError(val.clone()));
        }
        self.val = Some(input);
        Ok(())
    }

    /// Get nested value from tree given key
    #[must_use]
    pub fn get<K: Into<TagKey>>(&self, key: K) -> Option<&TagTreeVal> {
        self.tree.as_ref()?.get(key)
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

pub trait TagsWrite {
    ///
    /// # Errors
    ///
    /// If duplicate key is inserted.
    ///
    fn checked_insert<K: Into<TagKey>, V: Into<String>>(
        &mut self,
        k: K,
        v: V,
    ) -> Result<(), DuplicateKeyError>;
}

impl TagsWrite for Tags {
    ///
    /// # Errors
    ///
    /// If duplicate key is inserted.   
    ///
    fn checked_insert<K: Into<TagKey>, V: Into<String>>(
        &mut self,
        k: K,
        v: V,
    ) -> Result<(), DuplicateKeyError> {
        let tag_key = k.into();
        let key = tag_key.as_str();
        let val: String = v.into();
        self.map
            .insert(key.to_owned(), val.clone())
            .map_or(Ok(()), |_| Err(DuplicateKeyError(key.to_owned())))?;
        self.tree.insert(key, val)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::items_after_statements)]
mod tests {
    use crate::tag::{TagKey, Tags};

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
        assert!(tags.is_any("foo", &["bar"]));
        assert!(tags.is_any("foo", &["foo", "bar"]));
        assert!(!tags.is_any("foo", &["foo"]));
        assert!(!tags.is_any("bar", &["foo", "bar"]));
        assert_eq!(tags.subset(&["foo"]).to_vec(), vec!["foo=bar"]);
        assert_eq!(
            tags.subset(&["foo", "abra"]).to_vec(),
            vec!["abra=cadabra", "foo=bar"]
        );
        assert_eq!(tags.subset(&["foo", "bar"]).to_vec(), vec!["foo=bar"]);
        assert!(tags.subset(&["bar"]).to_vec().is_empty());

        // Key interfaces
        const FOO_KEY: TagKey = TagKey::from("foo");
        assert!(tags.is(FOO_KEY, "bar"));
        assert!(!tags.is(FOO_KEY, "foo"));
        assert_eq!(tags.subset(&[FOO_KEY]).to_vec(), vec!["foo=bar"]);
        assert!(tags.is(FOO_KEY + "multi" + "key", "value"));
        let foo_key = FOO_KEY + "multi" + "key";
        assert!(tags.is(&foo_key, "value"));

        // Tree interfaces
        let tree = tags.tree();

        let abra = tree.get("abra");
        assert!(abra.is_some());
        let abra = abra.unwrap();
        assert_eq!(abra.val(), Some("cadabra"));
        assert!(abra.tree().is_none());

        let foo = tree.get(FOO_KEY);
        assert!(foo.is_some());
        let foo = foo.unwrap();
        assert_eq!(foo.val(), Some("bar"));
        assert!(foo.tree().is_some());

        let multi = foo.get("multi:key");
        assert!(multi.is_some());
        let multi = multi.unwrap();
        assert_eq!(multi.val(), Some("value"));
        assert!(multi.tree().is_none());
        assert_eq!(
            multi.val(),
            foo.get("multi").unwrap().get("key").unwrap().val()
        );

        // TODO: Multi Value
    }
}
