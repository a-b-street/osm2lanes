use std::borrow::Borrow;
use std::ops::Deref;

use kstring::KString;

/// A part of in OSM tag key.
/// Must never contain a `:`
/// ```
/// use osm_tags::TagKeyPart;
/// const example_part: TagKeyPart = TagKeyPart::from_static("example");
/// assert_eq!(example_part.as_str(), "example");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagKeyPart(KString);

impl TagKeyPart {
    /// Must not contain a `:`, as this cannot be handle at const time
    #[must_use]
    pub const fn from_static(string: &'static str) -> Self {
        Self(KString::from_static(string))
    }

    /// Must not contain a `:`, as this cannot be handle at const time
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for TagKeyPart {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Deref for TagKeyPart {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for TagKeyPart {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&'static str> for TagKeyPart {
    fn from(string: &'static str) -> Self {
        Self::from_static(string)
    }
}

impl std::ops::Add for TagKeyPart {
    type Output = TagKey;
    fn add(self, other: Self) -> Self::Output {
        Self::Output::new([self, other])
    }
}

impl std::ops::Add<&'static str> for TagKeyPart {
    type Output = TagKey;
    fn add(self, other: &'static str) -> Self::Output {
        Self::Output::new([self, other.into()])
    }
}

/// The key of an OSM tag stored as `:` separated parts
///
/// ```
/// use osm_tags::TagKey;
/// let example_key: TagKey = TagKey::from_ref("example:foo");
/// assert_eq!(example_key.to_string(), "example:foo");
/// ```
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct TagKey(pub(crate) Vec<TagKeyPart>);

impl TagKey {
    #[must_use]
    pub fn new<C>(collection: C) -> Self
    where
        C: IntoIterator,
        C::Item: Into<TagKeyPart>,
    {
        Self(collection.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn from_ref(string: &str) -> Self {
        Self(
            string
                .split(':')
                .map(KString::from_ref)
                .map(TagKeyPart)
                .collect(),
        )
    }

    #[must_use]
    pub fn parts(&self) -> &[TagKeyPart] {
        &self.0
    }
}

impl From<&String> for TagKey {
    fn from(string: &String) -> Self {
        Self::from_ref(string)
    }
}

impl From<&str> for TagKey {
    fn from(string: &str) -> Self {
        Self::from_ref(string)
    }
}

// TODO: this seems like a hack
impl From<&&str> for TagKey {
    fn from(string: &&str) -> Self {
        Self::from_ref(string)
    }
}

impl std::str::FromStr for TagKey {
    type Err = std::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_ref(s))
    }
}

impl From<TagKeyPart> for TagKey {
    fn from(part: TagKeyPart) -> Self {
        Self(vec![part])
    }
}

impl From<&TagKeyPart> for TagKey {
    fn from(part: &TagKeyPart) -> Self {
        Self(vec![part.clone()])
    }
}

// TODO: this is a hidden clone, which is incorrect
impl From<&TagKey> for TagKey {
    fn from(key: &TagKey) -> Self {
        key.clone()
    }
}

impl ToString for TagKey {
    fn to_string(&self) -> String {
        self.0.join(":")
    }
}

impl std::ops::Add for TagKey {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let val = format!("{}:{}", self.to_string(), other.to_string());
        Self::from(&val)
    }
}

impl std::ops::Add<&'static str> for TagKey {
    type Output = Self;
    fn add(self, other: &'static str) -> Self {
        self.add(TagKey::from(other))
    }
}
