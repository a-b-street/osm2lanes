use serde::Serialize;

use crate::road::Lane;
use crate::tags::{DuplicateKeyError, TagKey, Tags};

/// Tranformation Logic Issue
///
/// ```
/// use osm2lanes::transform::RoadMsg;
/// let _ = RoadMsg::deprecated_tag("foo", "bar");
/// let _ = RoadMsg::unsupported_tag("foo", "bar");
/// let _ = RoadMsg::unsupported_str("foo=bar because x and y");
/// ```
///
#[derive(Debug, Clone)]
pub enum RoadMsg {
    /// Deprecated OSM tags, with suggested alternative
    Deprecated {
        deprecated_tags: Tags,
        suggested_tags: Option<Tags>,
    },
    /// Tag combination that is unsupported, and may never be supported
    Unsupported {
        description: Option<String>,
        tags: Option<Tags>,
    },
    /// Tag combination that is known, but has yet to be implemented
    Unimplemented {
        description: Option<String>,
        tags: Option<Tags>,
    },
    /// Tag combination that is ambiguous, and may never be supported
    Ambiguous {
        description: Option<String>,
        tags: Option<Tags>,
    },
    /// Other issue
    Other {
        description: String,
        tags: Tags,
    },
    /// Internal error
    TagsDuplicateKey(DuplicateKeyError),
}

impl RoadMsg {
    pub fn deprecated_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        Self::Unsupported {
            description: None,
            tags: Some(Tags::from_str_pairs(&[[key.into().as_str(), val]]).unwrap()),
        }
    }
    pub fn unsupported_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        Self::Unsupported {
            description: None,
            tags: Some(Tags::from_str_pairs(&[[key.into().as_str(), val]]).unwrap()),
        }
    }
    pub fn unimplemented_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        Self::Unimplemented {
            description: None,
            tags: Some(Tags::from_str_pairs(&[[key.into().as_str(), val]]).unwrap()),
        }
    }
    pub fn unsupported_str(description: &str) -> Self {
        Self::Unsupported {
            description: Some(description.to_owned()),
            tags: None,
        }
    }
}

impl std::fmt::Display for RoadMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Deprecated {
                deprecated_tags, ..
            } => write!(
                f,
                "deprecated: {}",
                deprecated_tags.to_vec().as_slice().join(" ")
            ),
            Self::Unsupported { description, tags }
            | Self::Unimplemented { description, tags }
            | Self::Ambiguous { description, tags } => {
                let tags = tags.as_ref().map(|tags| tags.to_vec().as_slice().join(" "));
                let prefix = match self {
                    Self::Unsupported { .. } => "unsupported",
                    Self::Unimplemented { .. } => "unimplemented",
                    Self::Ambiguous { .. } => "ambiguous",
                    _ => unreachable!(),
                };
                match (description, tags) {
                    (None, None) => write!(f, "{}", prefix),
                    (Some(description), None) => {
                        write!(f, "{}: {}", prefix, description)
                    }
                    (None, Some(tags)) => write!(f, "{}: {}", prefix, tags),
                    (Some(description), Some(tags)) => {
                        write!(f, "{}: {}, {}", prefix, description, tags)
                    }
                }
            }
            Self::Other { description, .. } => write!(f, "{}", description),
            Self::TagsDuplicateKey(e) => e.fmt(f),
        }
    }
}

impl Serialize for RoadMsg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RoadWarnings(Vec<RoadMsg>);

impl RoadWarnings {
    pub fn new(msgs: Vec<RoadMsg>) -> Self {
        Self(msgs)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn push(&mut self, msg: RoadMsg) {
        self.0.push(msg)
    }
}

impl std::fmt::Display for RoadWarnings {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|warning| format!("Warning: {}", warning))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Debug, Clone, Serialize)]
/// Error for transformation
/// ```
/// use osm2lanes::transform::{RoadMsg, RoadError};
/// let msg: RoadMsg = RoadMsg::deprecated_tag("foo", "bar");
/// assert_eq!("\"unsupported: foo=bar\"", serde_json::to_string(&msg).unwrap());
/// let err: RoadError = msg.into();
/// assert_eq!("{\"error\":\"unsupported: foo=bar\"}", serde_json::to_string(&err).unwrap());
/// ```
pub enum RoadError {
    #[serde(rename = "error")]
    Msg(RoadMsg),
    #[serde(rename = "warnings")]
    Warnings(RoadWarnings),
    #[serde(rename = "round_trip")]
    RoundTrip,
}

impl RoadError {
    pub fn ambiguous_str(description: &str) -> Self {
        RoadMsg::unsupported_str(description).into()
    }
}

impl std::error::Error for RoadError {}

impl std::fmt::Display for RoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Msg(msg) => msg.fmt(f),
            Self::Warnings(warnings) => write!(f, "{} warnings", warnings.0.len()),
            Self::RoundTrip => write!(f, "lanes to tags cannot roundtrip"),
        }
    }
}

impl From<RoadMsg> for RoadError {
    fn from(msg: RoadMsg) -> Self {
        Self::Msg(msg)
    }
}

impl From<RoadWarnings> for RoadError {
    fn from(warnings: RoadWarnings) -> Self {
        Self::Warnings(warnings)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Lanes {
    pub lanes: Vec<Lane>,
    pub warnings: RoadWarnings,
}
