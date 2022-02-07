#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;

    use serde::Deserialize;

    use crate::road::{Lane, LanePrintable, Marking, Road};
    use crate::tags::Tags;
    use crate::transform::{
        lanes_to_tags, tags_to_lanes, Lanes, LanesToTagsConfig, RoadError, TagsToLanesConfig,
    };
    use crate::{DrivingSide, Locale};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RustTesting {
        Enabled(bool),
        WithOptions {
            separator: Option<bool>,
            ignore_warnings: Option<bool>,
        },
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
                        Some(RustTesting::WithOptions { separator, .. }) => {
                            separator.unwrap_or(true)
                        }
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

        fn ignore_warnings(&self) -> bool {
            match self.rust {
                None => false,
                Some(RustTesting::Enabled(_)) => false,
                Some(RustTesting::WithOptions {
                    ignore_warnings, ..
                }) => ignore_warnings.unwrap_or(false),
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
            serde_yaml::from_reader(BufReader::new(File::open("../../data/tests.yml").unwrap()))
                .expect("invalid json");
        let tests: Vec<TestCase> = tests.into_iter().filter(|test| test.is_enabled()).collect();

        assert!(
            tests.iter().all(|test| {
                let locale = Locale::builder().driving_side(test.driving_side).build();
                let lanes = tags_to_lanes(
                    &test.tags,
                    &locale,
                    &TagsToLanesConfig {
                        error_on_warnings: !test.ignore_warnings(),
                        ..TagsToLanesConfig::default()
                    },
                );
                let expected_road = test.expected_road();
                match lanes {
                    Ok(lanes) => {
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
                    }
                    Err(RoadError::Warnings(warnings)) => {
                        test.print();
                        println!("Expected:");
                        println!("    {}", stringify_lane_types(&expected_road));
                        println!("    {}", stringify_directions(&expected_road));
                        println!("{}", warnings);
                        println!();
                        false
                    }
                    Err(e) => {
                        test.print();
                        println!("Expected:");
                        println!("    {}", stringify_lane_types(&expected_road));
                        println!("    {}", stringify_directions(&expected_road));
                        println!("{}", e);
                        println!();
                        false
                    }
                }
            }),
            "test_from_data tags_to_lanes failed"
        );
    }

    #[test]
    fn test_roundtrip() {
        let tests: Vec<TestCase> =
            serde_yaml::from_reader(BufReader::new(File::open("../../data/tests.yml").unwrap()))
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
                        ..TagsToLanesConfig::default()
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
