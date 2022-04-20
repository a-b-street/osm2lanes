/// Modes of travel
///
mod bicycle;
pub(super) use bicycle::bicycle;

mod bus;
pub(super) use bus::{bus, BuswayScheme, LanesBusScheme};
mod foot_shoulder;
pub(super) use foot_shoulder::foot_and_shoulder;

mod parking;
pub(super) use parking::parking;

mod non_motorized;
pub(super) use non_motorized::non_motorized;
