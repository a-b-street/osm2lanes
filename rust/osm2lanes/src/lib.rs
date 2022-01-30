//! This crate transforms tags from an OpenStreetMap (OSM) way into a specification of the lanes on
//! that road.
//!
//! WARNING: The output specification and all of this code is just being prototyped. Don't depend
//! on anything yet.

use serde::{Deserialize, Serialize};

mod tags;
pub use self::tags::{Tags, TagsRead, TagsWrite};

mod locale;
pub use self::locale::{DrivingSide, Locale};

mod transform;
pub use self::transform::{
    lanes_to_tags, tags_to_lanes, Lanes, LanesToTagsConfig, RoadError, RoadMsg, RoadWarnings,
    TagsToLanesConfig,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub lanes: Vec<Lane>,
}

impl Road {
    pub fn has_separators(&self) -> bool {
        self.lanes.iter().any(|lane| lane.is_separator())
    }
}

impl Road {
    pub fn total_width(&self, locale: &Locale) -> Metre {
        self.lanes
            .iter()
            .map(|lane| match lane {
                Lane::Separator { markings } => markings
                    .iter()
                    .map(|marking| marking.width.unwrap_or(Marking::DEFAULT_WIDTH))
                    .sum::<Metre>(),
                Lane::Travel { designated, .. } => locale.default_width(designated),
                _ => Lane::DEFAULT_WIDTH,
            })
            .sum::<Metre>()
    }
}

/// A single lane
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lane {
    #[serde(rename = "travel")]
    Travel {
        // TODO, we could make this non-optional, but remove the field for designated=foot?
        direction: Option<LaneDirection>,
        designated: LaneDesignated,
    },
    #[serde(rename = "parking")]
    Parking {
        direction: LaneDirection,
        designated: LaneDesignated,
    },
    #[serde(rename = "shoulder")]
    Shoulder,
    #[serde(rename = "separator")]
    Separator { markings: Vec<Marking> },
    // #[serde(rename = "construction")]
    // Construction,
}

impl Lane {
    pub const DEFAULT_WIDTH: Metre = Metre(3.5);
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marking {
    pub style: MarkingStyle,
    pub width: Option<Metre>,
    pub color: Option<MarkingColor>,
}

impl Marking {
    const DEFAULT_WIDTH: Metre = Metre::new(0.2);
    const DEFAULT_SPACE: Metre = Metre::new(0.1);
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarkingStyle {
    #[serde(rename = "solid_line")]
    SolidLine,
    #[serde(rename = "broken_line")]
    BrokenLine,
    #[serde(rename = "dashed_line")]
    DashedLine,
    #[serde(rename = "dotted_line")]
    DottedLine,
    // #[serde(rename = "gore_chevron")]
    // GoreChevron,
    // #[serde(rename = "diagnoal_hatched")]
    // DiagonalCross,
    // #[serde(rename = "criss_cross")]
    // CrissCross,
    // #[serde(rename = "solid_fill")]
    // SolidFill,
    #[serde(rename = "no_fill")]
    NoFill,
    // up and down are left to right
    #[serde(rename = "kerb_up")]
    KerbUp,
    #[serde(rename = "kerb_down")]
    KerbDown,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Metre(f64);

impl Metre {
    pub const fn new(val: f64) -> Self {
        Self(val)
    }
    pub const fn val(&self) -> f64 {
        self.0
    }
}

impl std::ops::Add for Metre {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}
impl std::ops::AddAssign for Metre {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0 + other.0);
    }
}
impl std::ops::Mul<Metre> for f64 {
    // The division of rational numbers is a closed operation.
    type Output = Metre;
    fn mul(self, other: Metre) -> Self::Output {
        Metre::new(self * other.val())
    }
}
impl std::iter::Sum for Metre {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Metre>,
    {
        Self(iter.map(|m| m.0).sum())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarkingColor {
    #[serde(rename = "white")]
    White,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "red")]
    Red,
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
            Self::Shoulder => 'S',
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
            Self::Shoulder => '🛆',
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

impl MarkingStyle {
    pub fn as_utf8(&self) -> char {
        match self {
            Self::SolidLine => '|',
            Self::BrokenLine => '¦',
            Self::DashedLine => ':',
            Self::DottedLine => '᛫',
            Self::KerbDown => '\\',
            Self::KerbUp => '/',
            Self::NoFill => ' ',
        }
    }
}

impl LanePrintable for MarkingColor {
    fn as_ascii(&self) -> char {
        match self {
            Self::White => 'w',
            Self::Yellow => 'y',
            Self::Red => 'r',
        }
    }
    fn as_utf8(&self) -> char {
        self.as_ascii()
    }
}

#[cfg(test)]
mod tests {
    use self::transform::{lanes_to_tags, LanesToTagsConfig};
    use super::*;

    use std::fs::File;
    use std::io::BufReader;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RustTesting {
        Enabled(bool),
        WithOptions { separator: Option<bool> },
    }

    #[derive(Deserialize)]
    struct TestCase {
        // Metadata
        /// The OSM way unique identifier
        way_id: Option<i64>,
        link: Option<String>,
        driving_side: DrivingSide,
        comment: Option<String>,
        description: Option<String>,
        // Data
        tags: Tags,
        output: Vec<Lane>,
        // Skipping
        rust: Option<RustTesting>,
    }

    impl Road {
        /// Eq where None is treaty as always equal
        fn approx_eq(&self, other: &Self) -> bool {
            if self.lanes.len() != other.lanes.len() {
                return false;
            }
            self.lanes
                .iter()
                .zip(other.lanes.iter())
                .all(|(left, right)| left.approx_eq(right))
        }
    }

    impl Lane {
        /// Eq where None is treaty as always equal
        fn approx_eq(&self, other: &Self) -> bool {
            if let (Lane::Separator { markings: left }, Lane::Separator { markings: right }) =
                (self, other)
            {
                left.iter()
                    .zip(right.iter())
                    .all(|(left, right)| left.approx_eq(right))
            } else {
                self == other
            }
        }
    }

    impl Marking {
        /// Eq where None is treaty as always equal
        fn approx_eq(&self, other: &Self) -> bool {
            self.style == other.style
                && match (self.color, other.color) {
                    (None, None) | (Some(_), None) | (None, Some(_)) => true,
                    (Some(left), Some(right)) => left == right,
                }
                && match (self.width, other.width) {
                    (None, None) | (Some(_), None) | (None, Some(_)) => true,
                    (Some(left), Some(right)) => left == right,
                }
        }
    }

    impl DrivingSide {
        /// Three-letter abbreviation
        const fn as_tla(&self) -> &'static str {
            match self {
                Self::Right => "RHT",
                Self::Left => "LHT",
            }
        }
    }

    impl TestCase {
        fn print(&self) {
            if let Some(description) = self.description.as_ref() {
                println!(
                    "Description: {} ({})",
                    description,
                    self.driving_side.as_tla()
                );
            }
            if self.way_id.is_some() {
                println!(
                    "For input (example from https://www.openstreetmap.org/way/{}) with {}:",
                    self.way_id.unwrap(),
                    self.driving_side.as_tla(),
                );
            } else if self.link.is_some() {
                println!("For input (example from {}):", self.link.as_ref().unwrap());
            }
            if let Some(comment) = self.comment.as_ref() {
                println!("        Comment: {}", comment);
            }
        }

        fn is_enabled(&self) -> bool {
            match self.rust {
                None => true,
                Some(RustTesting::Enabled(b)) => b,
                Some(RustTesting::WithOptions { .. }) => true,
            }
        }

        fn is_lane_enabled(&self, lane: &Lane) -> bool {
            match lane {
                Lane::Separator { .. } => {
                    let separator_testing = match self.rust {
                        None => true,
                        Some(RustTesting::Enabled(b)) => b,
                        Some(RustTesting::WithOptions { separator }) => separator.unwrap_or(true),
                    };
                    separator_testing && self.expected_has_separators()
                }
                _ => true,
            }
        }

        fn expected_has_separators(&self) -> bool {
            self.output.iter().any(|lane| lane.is_separator())
        }

        fn expected_road(&self) -> Road {
            Road {
                lanes: self
                    .output
                    .iter()
                    .filter(|lane| self.is_lane_enabled(lane))
                    .cloned()
                    .collect(),
            }
        }
    }

    impl Lanes {
        fn into_road(self, test: &TestCase) -> Road {
            Road {
                lanes: self
                    .lanes
                    .into_iter()
                    .filter(|lane| test.is_lane_enabled(lane))
                    .collect(),
            }
        }
    }

    fn stringify_lane_types(road: &Road) -> String {
        let simple = road.lanes.iter().map(|l| l.as_ascii()).collect();
        if road.has_separators() {
            let separators = road
                .lanes
                .iter()
                .filter_map(|lane| {
                    if let Lane::Separator { markings } = lane {
                        Some(
                            markings
                                .iter()
                                .map(|m| m.color.map_or(' ', |m| m.as_utf8()))
                                .collect::<String>(),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .as_slice()
                .join(" ");
            format!("{}    {}", simple, separators)
        } else {
            simple
        }
    }

    fn stringify_directions(road: &Road) -> String {
        let simple = road
            .lanes
            .iter()
            .map(|lane| {
                if let Lane::Travel {
                    direction: Some(direction),
                    ..
                } = lane
                {
                    direction.as_utf8()
                } else {
                    ' '
                }
            })
            .collect();
        if road.has_separators() {
            let separators = road
                .lanes
                .iter()
                .filter_map(|lane| {
                    if let Lane::Separator { markings } = lane {
                        Some(
                            markings
                                .iter()
                                .map(|m| m.style.as_utf8())
                                .collect::<String>(),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .as_slice()
                .join(" ");
            format!("{}    {}", simple, separators)
        } else {
            simple
        }
    }

    #[test]
    fn test_from_data() {
        let tests: Vec<TestCase> =
            serde_json::from_reader(BufReader::new(File::open("../../data/tests.json").unwrap()))
                .expect("invalid json");
        let tests: Vec<TestCase> = tests.into_iter().filter(|test| test.is_enabled()).collect();

        assert!(
            tests.iter().all(|test| {
                let locale = Locale::builder().driving_side(test.driving_side).build();
                let lanes = tags_to_lanes(
                    &test.tags,
                    &locale,
                    &TagsToLanesConfig {
                        error_on_warnings: true,
                    },
                );
                let expected_road = test.expected_road();
                if let Ok(lanes) = lanes {
                    let actual_road = lanes.into_road(test);
                    if actual_road.approx_eq(&expected_road) {
                        true
                    } else {
                        test.print();
                        println!("Got:");
                        println!("    {}", stringify_lane_types(&actual_road));
                        println!("    {}", stringify_directions(&actual_road));
                        println!("Expected:");
                        println!("    {}", stringify_lane_types(&expected_road));
                        println!("    {}", stringify_directions(&expected_road));
                        println!();
                        false
                    }
                } else {
                    test.print();
                    println!("Expected:");
                    println!("    {}", stringify_lane_types(&expected_road));
                    println!("    {}", stringify_directions(&expected_road));
                    println!("Panicked:");
                    println!("{:#?}", lanes.unwrap_err());
                    println!();
                    false
                }
            }),
            "test_from_data tags_to_lanes failed"
        );
    }

    #[test]
    fn test_roundtrip() {
        let tests: Vec<TestCase> =
            serde_json::from_reader(BufReader::new(File::open("../../data/tests.json").unwrap()))
                .unwrap();
        let tests: Vec<TestCase> = tests.into_iter().filter(|test| test.is_enabled()).collect();

        assert!(
            tests.iter().all(|test| {
                let locale = Locale::builder().driving_side(test.driving_side).build();
                let input_road = test.expected_road();
                let tags = lanes_to_tags(
                    &test.output,
                    &locale,
                    &LanesToTagsConfig {
                        check_roundtrip: false,
                    },
                )
                .unwrap();
                let output_lanes = tags_to_lanes(
                    &tags,
                    &locale,
                    &TagsToLanesConfig {
                        error_on_warnings: false,
                    },
                )
                .unwrap();
                let output_road = output_lanes.into_road(test);
                if input_road.approx_eq(&output_road) {
                    true
                } else {
                    test.print();
                    println!("From:");
                    println!("    {}", stringify_lane_types(&input_road));
                    println!("    {}", stringify_directions(&input_road));
                    println!("Normalized OSM tags:");
                    for (k, v) in tags.map() {
                        println!("    {} = {}", k, v);
                    }
                    println!("Got:");
                    println!("    {}", stringify_lane_types(&output_road));
                    println!("    {}", stringify_directions(&output_road));
                    println!();
                    false
                }
            }),
            "test_roundtrip lanes_to_tags failed"
        );
    }
}
