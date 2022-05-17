use kstring::KString;

/// A representation for the key of an OSM tag
///
/// ```
/// use osm_tags::TagKey;
/// const example_key: TagKey = TagKey::from_static("example");
/// assert_eq!(example_key.as_str(), "example");
/// assert_eq!((example_key + "foo").as_str(), "example:foo");
/// ```
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct TagKey(KString);

impl TagKey {
    #[must_use]
    pub const fn from_static(string: &'static str) -> Self {
        Self(KString::from_static(string))
    }

    #[must_use]
    pub fn from_ref(string: &str) -> Self {
        Self(KString::from_ref(string))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for TagKey {
    fn from(string: String) -> Self {
        Self(KString::from_string(string))
    }
}

impl From<&String> for TagKey {
    fn from(string: &String) -> Self {
        Self(KString::from_ref(string))
    }
}

impl From<&'static str> for TagKey {
    fn from(string: &'static str) -> Self {
        Self::from_static(string)
    }
}

impl std::str::FromStr for TagKey {
    type Err = std::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(KString::from_ref(s)))
    }
}

impl AsRef<str> for TagKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::ops::Add for TagKey {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let val = format!("{}:{}", self.as_str(), other.as_str());
        Self::from(val)
    }
}

impl std::ops::Add<&'static str> for TagKey {
    type Output = Self;
    fn add(self, other: &'static str) -> Self {
        self.add(TagKey::from(other))
    }
}
