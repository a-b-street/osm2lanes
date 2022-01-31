use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateKeyError(String);

impl std::fmt::Display for DuplicateKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "duplicate tag key {}", self.0)
    }
}

/// A representation for a OSM tags key
#[derive(Clone)]
pub enum TagKey {
    Static(&'static str),
    String(String),
}

impl TagKey {
    pub const fn from(string: &'static str) -> Self {
        TagKey::Static(string)
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(v) => v,
            Self::String(v) => v.as_str(),
        }
    }
}

impl From<&'static str> for TagKey {
    fn from(string: &'static str) -> Self {
        TagKey::from(string)
    }
}

impl std::ops::Add for TagKey {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let val = format!("{}:{}", self.as_str(), other.as_str());
        TagKey::String(val)
    }
}

impl std::ops::Add<&'static str> for TagKey {
    type Output = Self;
    fn add(self, other: &'static str) -> Self {
        self.add(TagKey::from(other))
    }
}

/// A map from string keys to string values. This makes copies of strings for
/// convenience; don't use in performance sensitive contexts.
// BTreeMap chosen for deterministic serialization.
// We often need to compare output directly, so cannot tolerate reordering
// TODO: fix this in the serialization by having the keys sorted.
#[derive(Clone, Debug, Deserialize, Default, Serialize)]
pub struct Tags(BTreeMap<String, String>);

impl Tags {
    pub fn from_str_pairs(tags: &[[&str; 2]]) -> Result<Self, DuplicateKeyError> {
        let mut map = BTreeMap::new();
        for tag in tags {
            map.insert(tag[0].to_owned(), tag[1].to_owned())
                .map_or(Ok(()), |_| Err(DuplicateKeyError(tag[0].to_owned())))?;
        }
        Ok(Self(map))
    }

    pub fn new(map: BTreeMap<String, String>) -> Tags {
        Tags(map)
    }

    /// Expose inner map
    pub fn map(&self) -> &BTreeMap<String, String> {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|(k, v)| format!("{}={}", k.as_str(), v.as_str()))
            .collect::<Vec<String>>()
    }

    /// Get tree
    ///
    /// Parses colon separated keys like `cycleway:right:oneway` as a tree.
    ///
    /// ```
    /// use std::str::FromStr;
    /// use osm2lanes::Tags;
    /// let tags = Tags::from_str("foo=bar\na:b:c=foobar").unwrap();
    /// let tree = tags.tree();
    /// let a = tree.get("a");
    /// assert!(a.is_some());
    /// let a = a.unwrap();
    /// assert!(a.val().is_none());
    /// let c = a.get("b:c");
    /// assert!(c.is_some());
    /// let c = c.unwrap();
    /// assert_eq!(c.val(), Some("foobar"))
    /// ```
    pub fn tree(&self) -> TagTree {
        let mut tag_tree = TagTree::default();
        for (k, v) in self.0.iter() {
            tag_tree.insert(k, v.to_owned());
        }
        tag_tree
    }
}

impl FromStr for Tags {
    type Err = String;

    /// Parse tags from an '=' separated list
    ///
    /// ```
    /// use std::str::FromStr;
    /// use osm2lanes::Tags;
    /// use osm2lanes::TagsRead;
    /// let tags = Tags::from_str("foo=bar\nabra=cadabra").unwrap();
    /// assert_eq!(tags.get("foo"), Some("bar"));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut map = BTreeMap::new();
        for line in s.lines() {
            let (key, val) = line.split_once('=').ok_or("tag must be = separated")?;
            map.insert(key.to_owned(), val.to_owned());
        }
        Ok(Self(map))
    }
}

impl ToString for Tags {
    /// Return tags as an '=' separated list
    ///
    /// ```
    /// use std::str::FromStr;
    /// use std::string::ToString;
    /// use osm2lanes::Tags;
    /// use osm2lanes::TagsRead;
    /// let tags = Tags::from_str("foo=bar\nabra=cadabra").unwrap();
    /// assert_eq!(tags.to_string(), "abra=cadabra\nfoo=bar");
    /// ```
    fn to_string(&self) -> String {
        self.to_vec().as_slice().join("\n")
    }
}

#[derive(Clone, Default)]
pub struct TagTree(BTreeMap<String, TagTreeVal>);

impl TagTree {
    fn insert(&mut self, key: &str, val: String) {
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
                .get(TagKey::String(rest_key.to_owned()))
        } else {
            next
        }
    }
}

#[derive(Clone, Default)]
pub struct TagTreeVal {
    tree: Option<TagTree>,
    val: Option<String>,
}

impl TagTreeVal {
    fn insert(&mut self, key: Option<&str>, val: String) {
        match key {
            Some(key) => self
                .tree
                .get_or_insert_with(TagTree::default)
                .insert(key, val),
            None => self.set(val),
        }
    }
    fn set(&mut self, input: String) {
        if let Some(val) = &self.val {
            panic!("TagTreeVal already contains value {}", val);
        }
        self.val = Some(input);
    }
    pub fn get<K: Into<TagKey>>(&self, key: K) -> Option<&TagTreeVal> {
        self.tree.as_ref()?.get(key)
    }
    pub fn val(&self) -> Option<&str> {
        Some(self.val.as_ref()?.as_str())
    }
}

// TODO, shouldn't TagKey be passed by reference?
pub trait TagsRead {
    // Basic read operations
    fn get<T: Into<TagKey>>(&self, k: T) -> Option<&str>;
    fn is<T: Into<TagKey>>(&self, k: T, v: &str) -> bool;
    fn is_any<T: Into<TagKey>>(&self, k: T, values: &[&str]) -> bool;
    // Filtering
    /// Create a subset of the tags. Missing keys are ignored.
    fn subset<T>(&self, keys: &[T]) -> Self
    where
        T: Clone,
        T: Into<TagKey>;
}

impl TagsRead for Tags {
    fn get<T: Into<TagKey>>(&self, k: T) -> Option<&str> {
        self.0.get(k.into().as_str()).map(|v| v.as_str())
    }

    fn is<T: Into<TagKey>>(&self, k: T, v: &str) -> bool {
        self.get(k) == Some(v)
    }

    fn is_any<T: Into<TagKey>>(&self, k: T, values: &[&str]) -> bool {
        if let Some(v) = self.get(k) {
            values.contains(&v)
        } else {
            false
        }
    }

    // TODO, find a way to do this without so many clones
    fn subset<T>(&self, keys: &[T]) -> Self
    where
        T: Clone,
        T: Into<TagKey>,
    {
        let mut map = Self::default();
        for key in keys {
            let tag_key: TagKey = key.clone().into();
            if let Some(val) = self.get(tag_key.clone()) {
                assert!(map
                    .0
                    .insert(tag_key.as_str().to_owned(), val.to_owned())
                    .is_none());
            }
        }
        map
    }
}

pub trait TagsWrite {
    /// Returns the old value of this key, if it was already present.
    fn insert<K: Into<TagKey>, V: Into<String>>(&mut self, k: K, v: V) -> Option<String>;
    fn checked_insert<K: Into<TagKey> + Copy, V: Into<String>>(
        &mut self,
        k: K,
        v: V,
    ) -> Result<(), DuplicateKeyError>;
}

impl TagsWrite for Tags {
    fn insert<K: Into<TagKey>, V: Into<String>>(&mut self, k: K, v: V) -> Option<String> {
        self.0.insert(k.into().as_str().to_owned(), v.into())
    }
    fn checked_insert<K: Into<TagKey> + Copy, V: Into<String>>(
        &mut self,
        k: K,
        v: V,
    ) -> Result<(), DuplicateKeyError> {
        self.insert(k, v).map_or(Ok(()), |_| {
            Err(DuplicateKeyError(k.into().as_str().to_owned()))
        })
    }
}
