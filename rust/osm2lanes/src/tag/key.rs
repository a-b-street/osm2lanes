/// A representation for the key of an OSM tag
///
/// ```
/// use osm2lanes::tag::TagKey;
/// const example_key: TagKey = TagKey::from("example");
/// assert_eq!(example_key.as_str(), "example");
/// assert_eq!((example_key + "foo").as_str(), "example:foo");
/// ```
#[derive(Clone)]
pub struct TagKey(TagKeyEnum);

#[derive(Clone)]
enum TagKeyEnum {
    Static(&'static str),
    String(String),
}

impl TagKey {
    pub const fn from(string: &'static str) -> Self {
        TagKey(TagKeyEnum::Static(string))
    }
    pub fn as_str(&self) -> &str {
        match &self.0 {
            TagKeyEnum::Static(v) => v,
            TagKeyEnum::String(v) => v.as_str(),
        }
    }
}

impl From<&'static str> for TagKey {
    fn from(string: &'static str) -> Self {
        TagKey::from(string)
    }
}

impl From<String> for TagKey {
    fn from(string: String) -> Self {
        TagKey(TagKeyEnum::String(string))
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
        val.into()
    }
}

impl std::ops::Add<&'static str> for TagKey {
    type Output = Self;
    fn add(self, other: &'static str) -> Self {
        self.add(TagKey::from(other))
    }
}
