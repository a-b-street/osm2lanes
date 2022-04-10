use crate::metric::Metre;
use crate::road::Style;

/// Semantic speed class
enum _SpeedClass {
    Walking,
    /// ~30kph / ~20mph
    Living,
    /// ~50kph / ~30mph
    Intra,
    /// ~80kph / ~50mph
    Inter,
    Max,
}

/// Semantic lane separator
enum _Separator {
    /// Into grass or dirt
    SoftEdge,
    /// Into a building or other hard surface
    HardEdge,
    /// Motorway (or other) shoulder
    Shoulder(_SpeedClass),
    /// Road paint for same direction
    Lane { speed: _SpeedClass },
    /// Road paint for opposite direction
    Centre { speed: _SpeedClass },
    /// Painted area
    Buffer { width: Metre, style: Style },
    /// Kerb step
    // TODO: solve directionality
    Kerb,
    /// Grassy verge
    Verge { width: Metre },
}
