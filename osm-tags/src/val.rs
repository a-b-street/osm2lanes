use std::ops::Deref;

/// A Tag Value
/// A String is used as a placeholder until `|` separated values are supported
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct TagVal(String);

impl TagVal {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for TagVal {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for TagVal {
    fn from(val: &str) -> Self {
        TagVal(val.to_owned())
    }
}

impl From<String> for TagVal {
    fn from(val: String) -> Self {
        TagVal(val)
    }
}
