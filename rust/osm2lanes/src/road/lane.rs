use serde::{Deserialize, Serialize};

use super::Markings;
use crate::{Locale, Metre};

/// A single lane
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lane {
    #[serde(rename = "travel")]
    Travel {
        // TODO, we could make this non-optional, but remove the field for designated=foot?
        direction: Option<LaneDirection>,
        designated: LaneDesignated,
        width: Option<Metre>,
    },
    #[serde(rename = "parking")]
    Parking {
        direction: LaneDirection,
        designated: LaneDesignated,
        width: Option<Metre>,
    },
    #[serde(rename = "shoulder")]
    Shoulder { width: Option<Metre> },
    #[serde(rename = "separator")]
    Separator { markings: Markings },
}

impl Lane {
    pub const DEFAULT_WIDTH: Metre = Metre::new(3.5);

    /// Width in metres
    pub fn width(&self, locale: &Locale) -> Metre {
        match self {
            Lane::Separator { markings } => markings.width(locale),
            Lane::Travel {
                width, designated, ..
            } => width.unwrap_or_else(|| locale.travel_width(designated)),
            // TODO: parking different from travel?
            Lane::Parking {
                width, designated, ..
            } => width.unwrap_or_else(|| locale.travel_width(designated)),
            Lane::Shoulder { width, .. } => width.unwrap_or(Lane::DEFAULT_WIDTH),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneDirection {
    #[serde(rename = "forward")]
    Forward,
    #[serde(rename = "backward")]
    Backward,
    #[serde(rename = "both")]
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LaneDesignated {
    // #[serde(rename = "any")]
    // Any,
    #[serde(rename = "foot")]
    Foot,
    #[serde(rename = "bicycle")]
    Bicycle,
    #[serde(rename = "motor_vehicle")]
    Motor,
    #[serde(rename = "bus")]
    Bus,
}

/// Display lane detail as printable characters
pub trait LanePrintable {
    fn as_ascii(&self) -> char;
    fn as_utf8(&self) -> char;
}

impl LanePrintable for Lane {
    fn as_ascii(&self) -> char {
        match self {
            Self::Travel {
                designated: LaneDesignated::Foot,
                ..
            } => 's',
            Self::Travel {
                designated: LaneDesignated::Bicycle,
                ..
            } => 'b',
            Self::Travel {
                designated: LaneDesignated::Motor,
                ..
            } => 'd',
            Self::Travel {
                designated: LaneDesignated::Bus,
                ..
            } => 'B',
            Self::Shoulder { .. } => 'S',
            Self::Parking { .. } => 'p',
            Self::Separator { .. } => '|',
        }
    }
    fn as_utf8(&self) -> char {
        match self {
            Self::Travel {
                designated: LaneDesignated::Foot,
                ..
            } => '🚶',
            Self::Travel {
                designated: LaneDesignated::Bicycle,
                ..
            } => '🚲',
            Self::Travel {
                designated: LaneDesignated::Motor,
                ..
            } => '🚗',
            Self::Travel {
                designated: LaneDesignated::Bus,
                ..
            } => '🚌',
            Self::Shoulder { .. } => '🛆',
            Self::Parking { .. } => '🅿',
            Self::Separator { .. } => '|',
        }
    }
}

impl LanePrintable for LaneDirection {
    fn as_ascii(&self) -> char {
        match self {
            Self::Forward => '^',
            Self::Backward => 'v',
            Self::Both => '|',
        }
    }
    fn as_utf8(&self) -> char {
        match self {
            Self::Forward => '↑',
            Self::Backward => '↓',
            Self::Both => '↕',
        }
    }
}
