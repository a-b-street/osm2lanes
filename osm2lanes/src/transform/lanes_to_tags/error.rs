use std::panic::Location;

use osm_tags::DuplicateKeyError;

use crate::transform::RoadError;

/// Lanes To Tags Transformation Logic Issue
///
/// ```
/// use osm2lanes::transform::LanesToTagsMsg;
/// let _ = LanesToTagsMsg::unimplemented("foobar");
/// ```
#[derive(Clone, Debug)]
pub struct LanesToTagsMsg {
    location: &'static Location<'static>,
    issue: LanesToTagsIssue,
}

#[derive(Clone, Debug)]
pub(in crate::transform::lanes_to_tags) enum LanesToTagsIssue {
    Unimplemented(String),
    TagsDuplicateKey(DuplicateKeyError),
    Roundtrip(Option<RoadError>),
}

impl std::fmt::Display for LanesToTagsMsg {
    #[allow(clippy::panic_in_result_fn)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.issue {
            LanesToTagsIssue::Unimplemented(description) => {
                write!(f, "unimplemented: '{}' - {}", description, self.location)
            },
            LanesToTagsIssue::TagsDuplicateKey(e) => write!(f, "{} - {}", e, self.location),
            LanesToTagsIssue::Roundtrip(None) => write!(f, "roundtrip - {}", self.location),
            LanesToTagsIssue::Roundtrip(Some(e)) => {
                write!(f, "roundtrip: {} - {}", e, self.location)
            },
        }
    }
}

impl std::error::Error for LanesToTagsMsg {}

impl LanesToTagsMsg {
    #[must_use]
    #[track_caller]
    pub fn unimplemented(description: &str) -> Self {
        LanesToTagsMsg {
            location: Location::caller(),
            issue: LanesToTagsIssue::Unimplemented(description.to_owned()),
        }
    }

    #[must_use]
    #[track_caller]
    pub fn roundtrip() -> Self {
        LanesToTagsMsg {
            location: Location::caller(),
            issue: LanesToTagsIssue::Roundtrip(None),
        }
    }
}

impl From<DuplicateKeyError> for LanesToTagsMsg {
    #[track_caller]
    fn from(e: DuplicateKeyError) -> Self {
        LanesToTagsMsg {
            location: Location::caller(),
            issue: LanesToTagsIssue::TagsDuplicateKey(e),
        }
    }
}

impl From<RoadError> for LanesToTagsMsg {
    #[track_caller]
    fn from(e: RoadError) -> Self {
        LanesToTagsMsg {
            location: Location::caller(),
            issue: LanesToTagsIssue::Roundtrip(Some(e)),
        }
    }
}
