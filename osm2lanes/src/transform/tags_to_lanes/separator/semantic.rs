use crate::metric::{Metre, Speed};
use crate::road::{Designated, Style};
use crate::transform::tags_to_lanes::Infer;

/// Semantic speed class
#[derive(Debug)]
pub enum SpeedClass {
    Walking,
    /// ~30kph / ~20mph
    Living,
    /// ~50kph / ~30mph
    Intra,
    /// ~80kph / ~50mph
    Inter,
    Max,
}

impl From<Speed> for SpeedClass {
    #[allow(clippy::panic)]
    fn from(s: Speed) -> Self {
        match s.kph() {
            s if (0.0_f64..15.0_f64).contains(&s) => Self::Walking,
            s if (15.0_f64..40.0_f64).contains(&s) => Self::Living,
            s if (40.0_f64..70.0_f64).contains(&s) => Self::Intra,
            s if (70.0_f64..100.0_f64).contains(&s) => Self::Inter,
            s if (100.0_f64..200.0_f64).contains(&s) => Self::Max,
            _ => panic!("unexpected speed {s}"),
        }
    }
}

/// Overtaking rules
///
/// Note: this does not take into account the local vehicle-specific rules,
/// e.g. for motorcycle filtering or overtaking tractors.
#[derive(Debug)]
pub enum Overtake {
    Permitted,
    _Prohibited,
}

impl Default for Overtake {
    fn default() -> Self {
        // fail-deadly, see README
        Self::Permitted
    }
}

/// Lane change rules
///
/// Note: this does not take into account the local vehicle-specific rules,
/// e.g. for motorcycle filtering or overtaking tractors.
#[derive(Debug)]
pub enum LaneChange {
    Permitted,
    _Prohibited,
}

impl Default for LaneChange {
    fn default() -> Self {
        // fail-deadly, see README
        Self::Permitted
    }
}

/// Overtaking rules
///
/// Note: this does not take into account the local vehicle-specific rules,
/// e.g. for motorcycle filtering or overtaking tractors.
#[derive(Debug)]
pub enum ParkingCondition {
    NoStopping,
}

/// Semantic lane separator
#[derive(Debug)]
pub enum Separator {
    /// Motorway (or other) shoulder
    Shoulder { speed: Infer<SpeedClass> },
    /// Road paint between same direction
    Lane {
        speed: Infer<SpeedClass>,
        change: LaneChange,
    },
    /// Road paint between opposite direction
    Centre {
        speed: Infer<SpeedClass>,
        overtake: Overtake,
        more_than_2_lanes: bool,
    },
    /// Road paint between different modes
    // TODO: solve directionality
    Modal {
        speed: Infer<SpeedClass>,
        change: LaneChange,
        inside: Designated,
        outside: Designated,
    },
    /// Painted area
    _Buffer { width: Metre, style: Style },
    /// Kerb step
    // TODO: solve directionality
    Kerb {
        // https://wiki.openstreetmap.org/wiki/Key:parking:condition
        parking_condition: Option<ParkingCondition>,
    },
    /// Grassy verge
    _Verge { width: Metre },
}

/// Semantic lane edge separator
#[derive(Debug)]
pub enum EdgeSeparator {
    /// Into grass or dirt
    // Soft,
    /// Into a building or other hard surface
    Hard {
        // https://wiki.openstreetmap.org/wiki/Key:parking:condition
        parking_condition: Option<ParkingCondition>,
    },
}
