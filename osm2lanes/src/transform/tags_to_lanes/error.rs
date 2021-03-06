use std::panic::Location;

use osm_tags::{DuplicateKeyError, TagKey, Tags};

use crate::transform::tags_to_lanes::LaneBuilder;

/// Tags to Lanes Transformation Logic Issue
///
/// ```
/// use osm2lanes::transform::TagsToLanesMsg;
/// let _ = TagsToLanesMsg::deprecated_tag("foo", "bar");
/// let _ = TagsToLanesMsg::unsupported_tag("foo", "bar");
/// let _ = TagsToLanesMsg::unsupported_str(String::from("foo=bar because x and y"));
/// let _ = TagsToLanesMsg::ambiguous_str("foo=bar because x and y");
/// ```
#[derive(Clone, Debug)]
pub struct TagsToLanesMsg {
    location: &'static Location<'static>,
    pub(in crate::transform) issue: TagsToLanesIssue,
}

#[derive(Clone, Debug)]
pub(crate) enum TagsToLanesIssue {
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
    /// Locale not used
    SeparatorLocaleUnused {
        inside: LaneBuilder,
        outside: LaneBuilder,
    },
    /// Locale not used
    SeparatorUnknown {
        inside: LaneBuilder,
        outside: LaneBuilder,
    },
    /// Internal errors
    TagsDuplicateKey(DuplicateKeyError),
    Internal(&'static str),
}

impl TagsToLanesMsg {
    #[must_use]
    #[track_caller]
    pub fn deprecated(deprecated: Tags, suggested: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Deprecated {
                deprecated_tags: deprecated,
                suggested_tags: Some(suggested),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn deprecated_tags(tags: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Deprecated {
                deprecated_tags: tags,
                suggested_tags: None,
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn deprecated_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Deprecated {
                deprecated_tags: Tags::from_pair(key, val),
                suggested_tags: None,
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unsupported(description: impl Into<String>, tags: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unsupported {
                description: Some(description.into()),
                tags: Some(tags),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unsupported_tags(tags: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unsupported {
                description: None,
                tags: Some(tags),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unsupported_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unsupported {
                description: None,
                tags: Some(Tags::from_pair(key, val)),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unsupported_str(description: impl Into<String>) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unsupported {
                description: Some(description.into()),
                tags: None,
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unimplemented(description: impl Into<String>, tags: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unimplemented {
                description: Some(description.into()),
                tags: Some(tags),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unimplemented_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unimplemented {
                description: None,
                tags: Some(Tags::from_pair(key, val)),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unimplemented_tags(tags: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Unimplemented {
                description: None,
                tags: Some(tags),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn ambiguous_tag<K: Into<TagKey>>(key: K, val: &str) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Ambiguous {
                description: None,
                tags: Some(Tags::from_pair(key, val)),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn ambiguous_tags(tags: Tags) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Ambiguous {
                description: None,
                tags: Some(tags),
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn ambiguous_str(description: impl Into<String>) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Ambiguous {
                description: Some(description.into()),
                tags: None,
            },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn separator_locale_unused(inside: LaneBuilder, outside: LaneBuilder) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::SeparatorLocaleUnused { inside, outside },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn separator_unknown(inside: LaneBuilder, outside: LaneBuilder) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::SeparatorUnknown { inside, outside },
        }
    }

    #[must_use]
    #[track_caller]
    pub fn internal(e: &'static str) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::Internal(e),
        }
    }
}

impl From<DuplicateKeyError> for TagsToLanesMsg {
    #[track_caller]
    fn from(e: DuplicateKeyError) -> Self {
        TagsToLanesMsg {
            location: Location::caller(),
            issue: TagsToLanesIssue::TagsDuplicateKey(e),
        }
    }
}

impl std::fmt::Display for TagsToLanesMsg {
    #[allow(clippy::panic_in_result_fn)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.issue {
            TagsToLanesIssue::Deprecated {
                deprecated_tags,
                suggested_tags,
            } => {
                if let Some(suggested_tags) = suggested_tags {
                    write!(
                        f,
                        "deprecated: replace '{}' with '{}' - {}",
                        deprecated_tags.to_vec().as_slice().join(" "),
                        suggested_tags.to_vec().as_slice().join(" "),
                        self.location,
                    )
                } else {
                    write!(
                        f,
                        "deprecated: '{}' - {}",
                        deprecated_tags.to_vec().as_slice().join(" "),
                        self.location,
                    )
                }
            },
            TagsToLanesIssue::Unsupported { description, tags }
            | TagsToLanesIssue::Unimplemented { description, tags }
            | TagsToLanesIssue::Ambiguous { description, tags } => {
                let tags = tags.as_ref().map(|tags| {
                    let tags = tags.to_vec();
                    if tags.is_empty() {
                        String::from("no tags")
                    } else {
                        tags.as_slice().join(" ")
                    }
                });
                let prefix = match self.issue {
                    TagsToLanesIssue::Unsupported { .. } => "unsupported",
                    TagsToLanesIssue::Unimplemented { .. } => "unimplemented",
                    TagsToLanesIssue::Ambiguous { .. } => "ambiguous",
                    _ => unreachable!(),
                };
                match (description, tags) {
                    (None, None) => write!(f, "{}", prefix),
                    (Some(description), None) => {
                        write!(f, "{}: '{}'", prefix, description)
                    },
                    (None, Some(tags)) => write!(f, "{}: '{}' - {}", prefix, tags, self.location),
                    (Some(description), Some(tags)) => {
                        write!(
                            f,
                            "{}: '{}' - '{}' - {}",
                            prefix, description, tags, self.location
                        )
                    },
                }
            },
            TagsToLanesIssue::SeparatorLocaleUnused { inside, outside } => {
                write!(
                    f,
                    "default separator may not match locale for {:?} to {:?} - {}",
                    inside, outside, self.location,
                )
            },
            TagsToLanesIssue::SeparatorUnknown { inside, outside } => {
                write!(
                    f,
                    "unknown separator for {:?} to {:?} - {}",
                    inside, outside, self.location
                )
            },
            TagsToLanesIssue::TagsDuplicateKey(e) => write!(f, "{} - {}", e, self.location),
            TagsToLanesIssue::Internal(e) => write!(f, "{} - {}", e, self.location),
        }
    }
}

impl std::error::Error for TagsToLanesMsg {}

#[cfg(feature = "serde")]
impl serde::Serialize for TagsToLanesMsg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
