use crate::metric::{Metre, Speed};
use crate::road::{Designated, Style};
use crate::transform::tags_to_lanes::Infer;

/// Semantic speed class
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
    fn from(s: Speed) -> Self {
        match s.kph() {
            s if s > 0.0_f64 && s <= 10.0_f64 => Self::Walking,
            s if s > 10.0_f64 && s < 40.0_f64 => Self::Living,
            s if s >= 40.0_f64 && s < 70.0_f64 => Self::Intra,
            s if s >= 70.0_f64 && s < 100.0_f64 => Self::Inter,
            s if s >= 100.0_f64 && s < 200.0_f64 => Self::Max,
            _ => panic!("unexpected speed {s}"),
        }
    }
}

/// Overtaking rules
///
/// Note: this does not take into account the local vehicle-specific rules,
/// e.g. for motorcycle filtering or overtaking tractors.
pub enum Overtaking {
    Permitted,
    Prohibited,
}

impl Default for Overtaking {
    fn default() -> Self {
        // fail-deadly, see README
        Self::Permitted
    }
}

/// Semantic lane separator
pub enum Separator {
    /// Into grass or dirt
    SoftEdge,
    /// Into a building or other hard surface
    HardEdge,
    /// Motorway (or other) shoulder
    Shoulder { speed: Infer<SpeedClass> },
    /// Road paint between same direction
    Lane {
        speed: Infer<SpeedClass>,
        overtaking: Overtaking,
    },
    /// Road paint between opposite direction
    Centre {
        speed: Infer<SpeedClass>,
        overtaking: Overtaking,
    },
    /// Road paint between different modes
    // TODO: solve directionality
    Modal {
        inside: Designated,
        outside: Designated,
    },
    /// Painted area
    Buffer { width: Metre, style: Style },
    /// Kerb step
    // TODO: solve directionality
    Kerb,
    /// Grassy verge
    Verge { width: Metre },
}
