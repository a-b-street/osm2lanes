use serde::Deserialize;

use crate::locale::DrivingSide;
use crate::road::{Lane, Road};
use crate::tag::{Highway, HighwayType, Tags};

#[derive(Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum RustTesting {
    Enabled(bool),
    WithOptions {
        separator: Option<bool>,
        expect_warnings: Option<bool>,
    },
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expected {
    Road(Road),
    // TODO: deprecated
    Output(Vec<Lane>),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Deserialize)]
pub struct TestCase {
    // Metadata
    /// The OSM way unique identifier
    pub way_id: Option<i64>,
    /// Relevant link
    pub link: Option<String>,
    /// Comment on test case
    pub comment: Option<String>,
    /// Description of test case
    pub description: Option<String>,

    /// List as a named example in the web app, with the given name
    example: Option<String>,

    // Config and Locale
    pub driving_side: DrivingSide,
    #[serde(rename = "ISO 3166-2")]
    pub iso_3166_2: Option<String>,

    /// Data
    pub tags: Tags,
    #[serde(flatten)]
    pub expected: Expected,

    pub rust: Option<RustTesting>,
}

impl TestCase {
    /// Lanes of expected output
    fn lanes(&self) -> &Vec<Lane> {
        match &self.expected {
            Expected::Road(road) => &road.lanes,
            Expected::Output(lanes) => lanes,
        }
    }
    /// Road of expected output
    #[must_use]
    pub fn road(&self) -> Road {
        match &self.expected {
            Expected::Road(road) => road.clone(),
            Expected::Output(lanes) => Road {
                highway: Highway::active(HighwayType::UnknownRoad),
                lanes: lanes.clone(),
            },
        }
    }
    /// Test case is enabled, true by default
    fn test_enabled(&self) -> bool {
        match self.rust {
            Some(RustTesting::Enabled(b)) => b,
            None | Some(RustTesting::WithOptions { .. }) => true,
        }
    }
    /// Test case must have warnings
    #[must_use]
    pub fn test_expects_warnings(&self) -> bool {
        match self.rust {
            None | Some(RustTesting::Enabled(_)) => false,
            Some(RustTesting::WithOptions {
                expect_warnings, ..
            }) => expect_warnings.unwrap_or(false),
        }
    }
    /// Test case expects matching separators
    #[must_use]
    pub fn test_include_separators(&self) -> bool {
        match self.rust {
            None => true,
            Some(RustTesting::Enabled(b)) => b,
            Some(RustTesting::WithOptions { separator, .. }) => separator.unwrap_or(true),
        }
    }
    /// Expected lanes include separator
    #[must_use]
    pub fn expected_has_separators(&self) -> bool {
        self.lanes().iter().any(Lane::is_separator)
    }
    /// Exemplary
    #[must_use]
    pub fn example(&self) -> Option<&str> {
        self.example.as_deref()
    }
}

impl std::fmt::Display for TestCase {
    #[allow(clippy::panic, clippy::restriction)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let way_id = self.way_id.map(|id| id.to_string());
        let names: [Option<&str>; 3] = [
            way_id.as_deref(),
            self.link.as_deref(),
            self.description.as_deref(),
        ];
        assert!(names.iter().any(Option::is_some), "invalid test case");
        write!(
            f,
            "{}",
            names
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .as_slice()
                .join(" ")
        )
    }
}

/// Get Test Cases from tests.yml
#[must_use]
pub fn get_tests() -> Vec<TestCase> {
    let tests: Vec<TestCase> = serde_yaml::from_str(include_str!("../../data/tests.yml"))
        .expect("invalid yaml in data/tests.yml");
    let tests: Vec<TestCase> = tests.into_iter().filter(TestCase::test_enabled).collect();
    tests
}

#[cfg(test)]
mod tests {

    use assert_json_diff::assert_json_eq;

    use super::*;
    use crate::locale::{DrivingSide, Locale};
    use crate::road::{Lane, Marking, Printable, Road};
    use crate::tag::Highway;
    use crate::transform::{
        lanes_to_tags, tags_to_lanes, LanesToTagsConfig, RoadError, RoadFromTags, RoadWarnings,
        TagsToLanesConfig,
    };

    static LOG_INIT: std::sync::Once = std::sync::Once::new();

    fn approx_eq<T: PartialEq>(actual: &Option<T>, expected: &Option<T>) -> bool {
        match (actual, expected) {
            (None, None) | (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(actual), Some(expected)) => actual == expected,
        }
    }

    impl Road {
        /// Eq where None is treaty as always equal
        fn approx_eq(&self, expected: &Self) -> bool {
            if self.lanes.len() != expected.lanes.len() {
                return false;
            }
            self.lanes
                .iter()
                .zip(expected.lanes.iter())
                .all(|(actual, expected)| actual.approx_eq(expected))
        }
    }

    impl Lane {
        /// Eq where None is treaty as always equal
        fn approx_eq(&self, expected: &Self) -> bool {
            #[allow(clippy::unnested_or_patterns)]
            match (self, expected) {
                (
                    Lane::Separator {
                        markings: actual, ..
                    },
                    Lane::Separator {
                        markings: expected, ..
                    },
                ) => actual
                    .iter()
                    .zip(expected.iter())
                    .all(|(actual, expected)| actual.approx_eq(expected)),
                (
                    Lane::Travel {
                        designated: actual_designated,
                        direction: actual_direction,
                        width: actual_width,
                        max_speed: actual_max_speed,
                        access: actual_access,
                    },
                    Lane::Travel {
                        designated: expected_designated,
                        direction: expected_direction,
                        width: expected_width,
                        max_speed: expected_max_speed,
                        access: expected_access,
                    },
                ) => {
                    actual_designated == expected_designated
                        && actual_direction == expected_direction
                        && approx_eq(actual_width, expected_width)
                        && approx_eq(actual_max_speed, expected_max_speed)
                        && approx_eq(actual_access, expected_access)
                },
                (
                    Lane::Parking {
                        designated: actual_designated,
                        direction: actual_direction,
                        width: actual_width,
                    },
                    Lane::Parking {
                        designated: expected_designated,
                        direction: expected_direction,
                        width: expected_width,
                    },
                ) => {
                    actual_designated == expected_designated
                        && actual_direction == expected_direction
                        && approx_eq(actual_width, expected_width)
                },
                (
                    Lane::Shoulder {
                        width: actual_width,
                    },
                    Lane::Shoulder {
                        width: expected_width,
                    },
                ) => approx_eq(actual_width, expected_width),
                (actual, expected) => actual == expected,
            }
        }
    }

    impl Marking {
        /// Eq where None is treaty as always equal
        fn approx_eq(&self, expected: &Self) -> bool {
            self.style == expected.style
                && approx_eq(&self.color, &expected.color)
                && approx_eq(&self.width, &expected.width)
        }
    }

    impl DrivingSide {
        /// Three-letter abbreviation
        const fn as_tla(self) -> &'static str {
            match self {
                Self::Right => "RHT",
                Self::Left => "LHT",
            }
        }
    }

    impl TestCase {
        fn print(&self) {
            if let Some(description) = self.description.as_ref() {
                println!("Description: {}", description);
            }
            if self.way_id.is_some() {
                println!(
                    "For input (example from https://www.openstreetmap.org/way/{}):",
                    self.way_id.unwrap(),
                );
            } else if self.link.is_some() {
                println!("For input (example from {}):", self.link.as_ref().unwrap());
            }
            println!(
                "    Driving({}) - Separators({}/{}) - Warnings({})",
                self.driving_side.as_tla(),
                self.test_include_separators(),
                self.expected_has_separators(),
                self.test_expects_warnings(),
            );
            if let Some(comment) = self.comment.as_ref() {
                println!("        Comment: {}", comment);
            }
        }

        fn is_lane_enabled(&self, lane: &Lane) -> bool {
            match lane {
                Lane::Separator { .. } => {
                    self.test_include_separators() && self.expected_has_separators()
                },
                _ => true,
            }
        }

        fn expected_road(&self) -> Road {
            Road {
                lanes: self
                    .lanes()
                    .iter()
                    .filter(|lane| self.is_lane_enabled(lane))
                    .cloned()
                    .collect(),
                highway: Highway::from_tags(&self.tags).unwrap(),
            }
        }
    }

    impl RoadFromTags {
        /// Return a Road based upon a `RoadFromTags` with irrelevant parts filtered out.
        fn into_filtered_road(self, test: &TestCase) -> (Road, RoadWarnings) {
            (
                Road {
                    lanes: self
                        .road
                        .lanes
                        .into_iter()
                        .filter(|lane| test.is_lane_enabled(lane))
                        .collect(),
                    highway: self.road.highway,
                },
                self.warnings,
            )
        }
    }

    fn stringify_lane_types(road: &Road) -> String {
        let simple = road
            .lanes
            .iter()
            .map(|l| format!("{:<2}", l.as_ascii()))
            .collect();
        if road.has_separators() {
            let separators = road
                .lanes
                .iter()
                .filter_map(|lane| {
                    if let Lane::Separator { markings, .. } = lane {
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
                format!("{:^2}", {
                    // TODO: direction on lane parking
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
            })
            .collect();
        if road.has_separators() {
            let separators = road
                .lanes
                .iter()
                .filter_map(|lane| {
                    if let Lane::Separator { markings, .. } = lane {
                        Some(
                            markings
                                .iter()
                                .map(|m| format!("{:^1}", m.style.as_utf8()))
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

    fn env_logger_init() {
        LOG_INIT.call_once(|| {
            env_logger::builder().is_test(true).init();
        });
    }

    #[test]
    fn test_from_data() {
        env_logger_init();
        let tests = get_tests();

        for test in &tests {
            let locale = Locale::builder()
                .driving_side(test.driving_side)
                .iso_3166_option(test.iso_3166_2.as_deref())
                .build();
            let road_from_tags = tags_to_lanes(
                &test.tags,
                &locale,
                &TagsToLanesConfig {
                    include_separators: test.test_include_separators()
                        && test.expected_has_separators(),
                    ..TagsToLanesConfig::default()
                },
            );
            let expected_road = test.expected_road();
            match road_from_tags {
                Ok(road_from_tags) => {
                    let (actual_road, warnings) = road_from_tags.into_filtered_road(test);
                    if actual_road.approx_eq(&expected_road) {
                        if test.test_expects_warnings() && warnings.is_empty() {
                            test.print();
                            println!("Expected warnings. Try removing `expect_warnings`.");
                            println!();
                            panic!("tags_to_lanes expected warnings");
                        } else if !test.test_expects_warnings() && !warnings.is_empty() {
                            test.print();
                            println!("Expected:");
                            println!("    {}", stringify_lane_types(&expected_road));
                            println!("    {}", stringify_directions(&expected_road));
                            println!("{}", warnings);
                            println!();
                            panic!("tags_to_lanes has warnings");
                        }
                    } else {
                        test.print();
                        println!("Got:");
                        println!("    {}", stringify_lane_types(&actual_road));
                        println!("    {}", stringify_directions(&actual_road));
                        println!("Expected:");
                        println!("    {}", stringify_lane_types(&expected_road));
                        println!("    {}", stringify_directions(&expected_road));
                        println!("{}", warnings);
                        if stringify_lane_types(&actual_road)
                            == stringify_lane_types(&expected_road)
                            || stringify_directions(&actual_road)
                                == stringify_directions(&expected_road)
                        {
                            assert_json_eq!(actual_road, expected_road);
                        }
                        println!();
                        panic!("tags_to_lanes output mismatch");
                    }
                },
                Err(RoadError::Warnings(_warnings)) => unreachable!(),
                Err(e) => {
                    test.print();
                    println!("Expected:");
                    println!("    {}", stringify_lane_types(&expected_road));
                    println!("    {}", stringify_directions(&expected_road));
                    println!("{}", e);
                    println!();
                    panic!("tags_to_lanes error");
                },
            }
        }
    }

    #[test]
    fn test_roundtrip() {
        env_logger_init();
        let tests = get_tests();

        for test in &tests {
            let locale = Locale::builder()
                .driving_side(test.driving_side)
                .iso_3166_option(test.iso_3166_2.as_deref())
                .build();
            let input_road = test.expected_road();
            let tags = lanes_to_tags(
                &test.road(),
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
                    include_separators: test.test_include_separators()
                        && test.expected_has_separators(),
                    ..TagsToLanesConfig::default()
                },
            )
            .unwrap();
            let (output_road, warnings) = output_lanes.into_filtered_road(test);
            if !output_road.approx_eq(&input_road) {
                test.print();
                println!("From:");
                println!("    {}", stringify_lane_types(&input_road));
                println!("    {}", stringify_directions(&input_road));
                println!("Normalized OSM tags:");
                for (k, v) in tags.to_str_pairs() {
                    println!("    {} = {}", k, v);
                }
                println!("Got:");
                println!("    {}", stringify_lane_types(&output_road));
                println!("    {}", stringify_directions(&output_road));
                println!("{}", warnings);
                if stringify_lane_types(&input_road) == stringify_lane_types(&output_road)
                    || stringify_directions(&input_road) == stringify_directions(&output_road)
                {
                    assert_json_eq!(input_road, output_road);
                }
                println!();
                panic!("lanes_to_tags roundtrip mismatch")
            }
        }
    }
}
