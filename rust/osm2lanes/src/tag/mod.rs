use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod key;
pub use key::TagKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateKeyError(String);

impl std::fmt::Display for DuplicateKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "duplicate tag key {}", self.0)
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
    /// Construct from slice of pairs
    pub fn from_str_pairs(tags: &[[&str; 2]]) -> Result<Self, DuplicateKeyError> {
        let mut map = BTreeMap::new();
        for tag in tags {
            map.insert(tag[0].to_owned(), tag[1].to_owned())
                .map_or(Ok(()), |_| Err(DuplicateKeyError(tag[0].to_owned())))?;
        }
        Ok(Self(map))
    }

    /// Expose data as vector of pairs
    pub fn to_str_pairs(&self) -> Vec<[&str; 2]> {
        self.0
            .iter()
            .map(|(k, v)| [k.as_str(), v.as_str()])
            .collect::<Vec<[&str; 2]>>()
    }

    /// Vector of `=` separated strings
    pub fn to_vec(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|(k, v)| format!("{}={}", k.as_str(), v.as_str()))
            .collect::<Vec<String>>()
    }

    // TODO, shouldn't TagKey be passed by reference?
    /// Get value from tags given a key
    pub fn get<T: AsRef<str>>(&self, k: T) -> Option<&str> {
        self.0.get(k.as_ref()).map(|v| v.as_str())
    }

    // TODO, shouldn't TagKey be passed by reference?
    /// Return if tags key has value,
    /// return false if key does not exist.
    pub fn is<T: AsRef<str>>(&self, k: T, v: &str) -> bool {
        self.get(k) == Some(v)
    }

    // TODO, shouldn't TagKey be passed by reference?
    /// Return if tags key has any of the values,
    /// return false if the key does not exist.
    pub fn is_any<T: AsRef<str>>(&self, k: T, values: &[&str]) -> bool {
        if let Some(v) = self.get(k) {
            values.contains(&v)
        } else {
            false
        }
    }

    // TODO, find a way to do this without so many clones
    pub fn subset<T>(&self, keys: &[T]) -> Self
    where
        T: Clone,
        T: AsRef<str>,
    {
        let mut map = Self::default();
        for key in keys {
            if let Some(val) = self.get(key) {
                assert!(map
                    .0
                    .insert(key.as_ref().to_owned(), val.to_owned())
                    .is_none());
            }
        }
        map
    }

    // TODO: bake into the type
    /// Get tree
    ///
    /// Parses colon separated keys like `cycleway:right:oneway` as a tree.
    ///
    /// ```
    /// use std::str::FromStr;
    /// use osm2lanes::tag::Tags;
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
    /// use osm2lanes::tag::Tags;
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
    /// use osm2lanes::tag::Tags;
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
                .get::<TagKey>(rest_key.to_owned().into())
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
    /// Get nested value from tree given key
    pub fn get<K: Into<TagKey>>(&self, key: K) -> Option<&TagTreeVal> {
        self.tree.as_ref()?.get(key)
    }
    /// Get value of root
    pub fn val(&self) -> Option<&str> {
        Some(self.val.as_ref()?.as_str())
    }
    /// Get tree
    pub fn tree(&self) -> Option<&TagTree> {
        self.tree.as_ref()
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

#[cfg(test)]
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
