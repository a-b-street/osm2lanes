/// A representation for the key of an OSM tag
///
/// ```
/// use osm2lanes::tags::TagKey;
/// const example_key: TagKey = TagKey::from("example");
/// assert_eq!(example_key.as_str(), "example");
/// assert_eq!((example_key + "foo").as_str(), "example:foo");
/// ```
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
