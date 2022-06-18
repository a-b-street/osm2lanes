/// Modes of travel
use osm_tag_schemes::LaneDependentAccessError;

use super::TagsToLanesMsg;

mod bicycle;
pub(super) use bicycle::bicycle;
pub(super) use bicycle::cycleway::Variant as CyclewayVariant;

mod bus;
pub(super) use bus::{bus, BusLaneCount, BuswayScheme};

mod foot_shoulder;
pub(super) use foot_shoulder::foot_and_shoulder;

mod parking;

pub(super) use parking::parking;

mod non_motorized;
pub(super) use non_motorized::non_motorized;

impl From<LaneDependentAccessError<'_>> for TagsToLanesMsg {
    fn from(e: LaneDependentAccessError) -> Self {
        match e {
            LaneDependentAccessError::Unknown(key, val) => {
                TagsToLanesMsg::unsupported_tag(key, val)
            },
            LaneDependentAccessError::Conflict => {
                // TODO, more detail
                TagsToLanesMsg::unsupported_str("conflicting tags")
            },
        }
    }
}
