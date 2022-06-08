/// Modes of travel
///
mod bicycle;
pub(super) use bicycle::{bicycle, Variant as CyclewayVariant};

mod bus;
pub(super) use bus::{bus, BusLaneCount, BuswayScheme};

mod foot_shoulder;
pub(super) use foot_shoulder::foot_and_shoulder;

mod parking;
pub(super) use parking::parking;

mod non_motorized;
pub(super) use non_motorized::non_motorized;
