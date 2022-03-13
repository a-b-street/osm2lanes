#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;

    use assert_json_diff::assert_json_eq;
    use serde::Deserialize;

    use crate::road::{Lane, LanePrintable, Marking, Road};
    use crate::tag::{Highway, Tags};
    use crate::transform::{
        lanes_to_tags, tags_to_lanes, LanesToTagsConfig, RoadError, RoadFromTags, TagsToLanesConfig,
    };
    use crate::{DrivingSide, Locale};

    #[derive(Deserialize)]
    #[serde(untagged, deny_unknown_fields)]
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
        comment: Option<String>,
        description: Option<String>,

        // Config and Locale
        driving_side: DrivingSide,
        #[serde(rename = "ISO 3166-2")]
        iso_3166_2: Option<String>,

        /// Data
        tags: Tags,
        // TODO: add nesting or rename to lanes
        output: Vec<Lane>,
        /// Configure Rust Testing
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
            match (self, other) {
                (Lane::Separator { markings: left }, Lane::Separator { markings: right }) => left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| left.approx_eq(right)),
                (
                    Lane::Travel {
                        designated: left_designated,
                        direction: left_direction,
                        width: left_width,
                        max_speed: None,
                    },
                    Lane::Travel {
                        designated: right_designated,
                        direction: right_direction,
                        width: right_width,
                        max_speed: None,
                    },
                ) => {
                    left_designated == right_designated
                        && left_direction == right_direction
                        && match (left_width, right_width) {
                            (None, None) | (Some(_), None) | (None, Some(_)) => true,
                            (Some(left), Some(right)) => left == right,
                        }
                }
                (
                    Lane::Parking {
                        designated: left_designated,
                        direction: left_direction,
                        width: left_width,
                    },
                    Lane::Parking {
                        designated: right_designated,
                        direction: right_direction,
                        width: right_width,
                    },
                ) => {
                    left_designated == right_designated
                        && left_direction == right_direction
                        && match (left_width, right_width) {
                            (None, None) | (Some(_), None) | (None, Some(_)) => true,
                            (Some(left), Some(right)) => left == right,
                        }
                }
                (Lane::Shoulder { width: left_width }, Lane::Shoulder { width: right_width }) => {
                    match (left_width, right_width) {
                        (None, None) | (Some(_), None) | (None, Some(_)) => true,
                        (Some(left), Some(right)) => left == right,
                    }
                }
                (left, right) => left == right,
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
                self.include_separators(),
                self.expected_has_separators(),
                !self.ignore_warnings(),
            );
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

        /// Test specifies to include separators
        fn include_separators(&self) -> bool {
            match self.rust {
                None => true,
                Some(RustTesting::Enabled(b)) => b,
                Some(RustTesting::WithOptions { separator, .. }) => separator.unwrap_or(true),
            }
        }

        fn is_lane_enabled(&self, lane: &Lane) -> bool {
            match lane {
                Lane::Separator { .. } => {
                    self.include_separators() && self.expected_has_separators()
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
                highway: Highway::from_tags(&self.tags).unwrap(),
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

    impl RoadFromTags {
        /// Return a Road based upon a RoadFromTags with irrelevant parts filtered out.
        fn into_filtered_road(self, test: &TestCase) -> Road {
            Road {
                lanes: self
                    .road
                    .lanes
                    .into_iter()
                    .filter(|lane| test.is_lane_enabled(lane))
                    .collect(),
                highway: self.road.highway,
            }
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
                    if let Lane::Separator { markings } = lane {
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

    #[test]
    fn test_from_data() {
        let tests: Vec<TestCase> =
            serde_yaml::from_reader(BufReader::new(File::open("../../data/tests.yml").unwrap()))
                .expect("invalid json");
        let tests: Vec<TestCase> = tests.into_iter().filter(|test| test.is_enabled()).collect();

        assert!(
            tests.iter().all(|test| {
                let locale = Locale::builder()
                    .driving_side(test.driving_side)
                    .iso_3166_option(&test.iso_3166_2)
                    .build();
                let road_from_tags = tags_to_lanes(
                    &test.tags,
                    &locale,
                    &TagsToLanesConfig {
                        error_on_warnings: !test.ignore_warnings(),
                        include_separators: test.include_separators()
                            && test.expected_has_separators(),
                        ..TagsToLanesConfig::default()
                    },
                );
                let expected_road = test.expected_road();
                match road_from_tags {
                    Ok(road_from_tags) => {
                        let actual_road = road_from_tags.into_filtered_road(test);
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
                            if stringify_lane_types(&actual_road)
                                == stringify_lane_types(&expected_road)
                                || stringify_directions(&actual_road)
                                    == stringify_directions(&expected_road)
                            {
                                assert_json_eq!(actual_road, expected_road);
                            }
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
                let locale = Locale::builder()
                    .driving_side(test.driving_side)
                    .iso_3166_option(&test.iso_3166_2)
                    .build();
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
                        include_separators: test.include_separators()
                            && test.expected_has_separators(),
                        ..TagsToLanesConfig::default()
                    },
                )
                .unwrap();
                let output_road = output_lanes.into_filtered_road(test);
                if input_road.approx_eq(&output_road) {
                    true
                } else {
                    test.print();
                    println!("From:");
                    println!("    {}", stringify_lane_types(&input_road));
                    println!("    {}", stringify_directions(&input_road));
                    println!("Normalized OSM tags:");
                    for [k, v] in tags.to_str_pairs() {
                        println!("    {} = {}", k, v);
                    }
                    println!("Got:");
                    println!("    {}", stringify_lane_types(&output_road));
                    println!("    {}", stringify_directions(&output_road));
                    if stringify_lane_types(&input_road) == stringify_lane_types(&output_road)
                        || stringify_directions(&input_road) == stringify_directions(&output_road)
                    {
                        assert_json_eq!(input_road, output_road);
                    }
                    println!();
                    false
                }
            }),
            "test_roundtrip lanes_to_tags failed"
        );
    }
}
