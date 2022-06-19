use std::borrow::Borrow;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::Hash;

use osm_tags::{TagKey, Tags};
use strum::{EnumString, IntoStaticStr, ParseError};

use crate::keys;

/// <https://wiki.openstreetmap.org/wiki/Key:access#Lane_dependent_restrictions>
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Access {
    #[strum(serialize = "")]
    None,
    No,
    Yes,
    Designated,
}

impl Access {
    fn split(lanes: &str) -> Result<Vec<Self>, ParseError> {
        lanes.split('|').map(str::parse).collect()
    }
}

/// Get value from tags given a key
fn get_access<'tag, Q, O>(
    tags: &'tag Tags,
    key: &Q,
) -> Result<Option<Vec<Access>>, LaneDependentAccessError<'tag>>
where
    TagKey: Borrow<Q>,
    Q: Ord + Hash + Eq + ?Sized + ToOwned<Owned = O>,
    O: Into<TagKey>,
{
    match tags.get(key) {
        Some(s) => match Access::split(s) {
            Ok(access) => Ok(Some(access)),
            Err(_parse_error) => Err(LaneDependentAccessError::Unknown(keys::TRACK_TYPE, s)),
        },
        None => Ok(None),
    }
}

#[derive(Debug)]
pub enum LaneDependentAccess {
    LeftToRight(Vec<Access>),
    Forward(Vec<Access>),
    Backward(Vec<Access>),
    ForwardBackward {
        forward: Vec<Access>,
        backward: Vec<Access>,
    },
}

#[derive(Debug)]
pub enum LaneDependentAccessError<'tag> {
    Unknown(TagKey, &'tag str),
    Conflict,
}

impl Display for LaneDependentAccessError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            LaneDependentAccessError::Conflict => write!(f, "conflicting tags"),
            LaneDependentAccessError::Unknown(key, val) => {
                write!(f, "unknown tag value {key}={val}")
            },
        }
    }
}

impl Error for LaneDependentAccessError<'_> {}

impl LaneDependentAccess {
    /// Parse access given | separated lanes in `Tags` given `TagKey`
    ///
    /// # Errors
    /// When the tags conflict.
    pub fn from_tags<'tag>(
        tags: &'tag Tags,
        key: &TagKey,
    ) -> Result<Option<Self>, LaneDependentAccessError<'tag>> {
        let key_forward = key + "forward";
        let key_backward = key + "backward";
        Ok(
            match (
                get_access(tags, key)?,
                (
                    get_access(tags, &key_forward)?,
                    get_access(tags, &key_backward)?,
                ),
            ) {
                (None, (Some(forward), None)) => Some(Self::Forward(forward)),
                (None, (None, Some(backward))) => Some(Self::Backward(backward)),
                (total, (Some(forward), Some(backward))) => {
                    if let Some(total) = total {
                        if forward.len().checked_add(backward.len()).unwrap() != total.len() {
                            return Err(LaneDependentAccessError::Conflict);
                        }
                        if forward
                            .iter()
                            .chain(backward.iter().rev())
                            .zip(total.iter())
                            .any(|(l, r)| l != r)
                        {
                            return Err(LaneDependentAccessError::Conflict);
                        }
                    }
                    Some(Self::ForwardBackward { forward, backward })
                },
                (Some(total), (forward, backward)) => {
                    if let Some(forward) = forward {
                        if total.iter().zip(forward.iter()).any(|(l, r)| l != r) {
                            return Err(LaneDependentAccessError::Conflict);
                        }
                    }
                    if let Some(backward) = backward {
                        if total
                            .iter()
                            .rev()
                            .zip(backward.iter().rev())
                            .any(|(l, r)| l != r)
                        {
                            return Err(LaneDependentAccessError::Conflict);
                        }
                    }
                    Some(Self::LeftToRight(total))
                },
                (None, (None, None)) => None,
            },
        )
    }
}
