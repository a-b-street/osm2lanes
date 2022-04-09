use celes::Country;

use super::{tags_to_lanes, RoadError, RoadMsg, TagsResult, TagsToLanesConfig};
use crate::locale::Locale;
use crate::metric::Speed;
use crate::road::{Designated, Direction, Lane};
use crate::tag::{DuplicateKeyError, Tags, TagsWrite};

impl std::convert::From<DuplicateKeyError> for RoadError {
    fn from(e: DuplicateKeyError) -> Self {
        RoadError::Msg(RoadMsg::TagsDuplicateKey(e))
    }
}

#[non_exhaustive]
pub struct Config {
    pub check_roundtrip: bool,
}

impl Config {
    #[must_use]
    pub fn new(check_roundtrip: bool) -> Self {
        Config { check_roundtrip }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            check_roundtrip: true,
        }
    }
}

impl Lane {
    fn is_shoulder(&self) -> bool {
        matches!(self, Lane::Shoulder { .. })
    }
}

/// Convert Lanes back to Tags
///
/// TODO: Take a Road struct instead of a slice of lanes
///
/// # Errors
///
/// Any of:
/// - internal error
/// - uninmplemented or unsupported functionality
/// - the OSM tag spec cannot represent the lanes
///
/// # Panics
///
/// Lanes slice is empty
#[allow(clippy::too_many_lines)]
pub fn lanes_to_tags(lanes: &[Lane], locale: &Locale, config: &Config) -> TagsResult {
    let mut tags = Tags::default();
    let mut oneway = false;
    tags.checked_insert("highway", "road")?; // TODO, add `highway` to `Lanes`

    // Lane Count
    let lane_count = lanes
        .iter()
        .filter(|lane| {
            matches!(
                lane,
                Lane::Travel {
                    designated: Designated::Motor | Designated::Bus,
                    ..
                }
            )
        })
        .count();
    tags.checked_insert("lanes", lane_count.to_string())?;
    // Oneway
    if lanes.iter().filter(|lane| lane.is_motor()).all(|lane| {
        matches!(
            lane,
            Lane::Travel {
                direction: Some(Direction::Forward),
                ..
            }
        )
    }) {
        tags.checked_insert("oneway", "yes")?;
        oneway = true;
    } else {
        // Forward
        let forward_lanes = lanes
            .iter()
            .filter(|lane| {
                matches!(
                    lane,
                    Lane::Travel {
                        designated: Designated::Motor | Designated::Bus,
                        direction: Some(Direction::Forward),
                        ..
                    }
                )
            })
            .count();
        tags.checked_insert("lanes:forward", forward_lanes.to_string())?;
        // Backward
        let backward_lanes = lanes
            .iter()
            .filter(|lane| {
                matches!(
                    lane,
                    Lane::Travel {
                        designated: Designated::Motor | Designated::Bus,
                        direction: Some(Direction::Backward),
                        ..
                    }
                )
            })
            .count();
        tags.checked_insert("lanes:backward", backward_lanes.to_string())?;
        // Both ways
        if lanes.iter().any(|lane| {
            matches!(
                lane,
                Lane::Travel {
                    designated: Designated::Motor,
                    direction: Some(Direction::Both),
                    ..
                }
            )
        }) {
            tags.checked_insert("lanes:both_ways", "1")?;
            if lane_count >= 3 {
                tags.checked_insert(
                    "turn:lanes:both_ways",
                    locale.driving_side.opposite().to_string(),
                )?;
            }
        }
    }
    // Shoulder
    match (
        lanes.first().unwrap().is_shoulder(),
        lanes.last().unwrap().is_shoulder(),
    ) {
        (false, false) => {
            // TODO do we want to always be explicit about this?
            tags.checked_insert("shoulder", "no")?;
        }
        (true, false) => {
            tags.checked_insert("shoulder", "left")?;
        }
        (false, true) => {
            tags.checked_insert("shoulder", "right")?;
        }
        (true, true) => tags.checked_insert("shoulder", "both")?,
    }
    // Pedestrian
    match (
        lanes.first().unwrap().is_foot(),
        lanes.last().unwrap().is_foot(),
    ) {
        (false, false) => {
            // TODO do we want to always be explicit about this?
            tags.checked_insert("sidewalk", "no")?;
        }
        (true, false) => tags.checked_insert("sidewalk", "left")?,
        (false, true) => tags.checked_insert("sidewalk", "right")?,
        (true, true) => tags.checked_insert("sidewalk", "both")?,
    }
    // Parking
    match (
        lanes
            .iter()
            .take_while(|lane| !lane.is_motor())
            .any(|lane| matches!(lane, Lane::Parking { .. })),
        lanes
            .iter()
            .skip_while(|lane| !lane.is_motor())
            .any(|lane| matches!(lane, Lane::Parking { .. })),
    ) {
        (false, false) => {}
        (true, false) => tags.checked_insert("parking:lane:left", "parallel")?,
        (false, true) => tags.checked_insert("parking:lane:right", "parallel")?,
        (true, true) => tags.checked_insert("parking:lane:both", "parallel")?,
    }
    // Cycleway
    {
        let left_cycle_lane: Option<Direction> = lanes
            .iter()
            .take_while(|lane| !lane.is_motor())
            .find(|lane| lane.is_bicycle())
            .and_then(Lane::direction);
        let right_cycle_lane: Option<Direction> = lanes
            .iter()
            .rev()
            .take_while(|lane| !lane.is_motor())
            .find(|lane| lane.is_bicycle())
            .and_then(Lane::direction);
        match (left_cycle_lane.is_some(), right_cycle_lane.is_some()) {
            (false, false) => {}
            (true, false) => tags.checked_insert("cycleway:left", "lane")?,
            (false, true) => tags.checked_insert("cycleway:right", "lane")?,
            (true, true) => tags.checked_insert("cycleway:both", "lane")?,
        }
        // https://wiki.openstreetmap.org/wiki/Key:cycleway:right:oneway
        {
            // if the way has oneway=yes and you are allowed to cycle against that oneway flow
            // also add oneway:bicycle=no to make it easier
            // for bicycle routers to see that the way can be used in two directions.
            if oneway
                && (left_cycle_lane.map_or(false, |direction| direction == Direction::Backward)
                    || right_cycle_lane.map_or(false, |direction| direction == Direction::Backward))
            {
                tags.checked_insert("oneway:bicycle", "no")?;
            }
            // indicate cycling traffic direction relative to the direction the osm way is oriented
            // yes: same direction
            // -1: contraflow
            // no: bidirectional
            match left_cycle_lane {
                Some(Direction::Forward) => {
                    tags.checked_insert("cycleway:left:oneway", "yes")?;
                }
                Some(Direction::Backward) => {
                    tags.checked_insert("cycleway:left:oneway", "-1")?;
                }
                Some(Direction::Both) => tags.checked_insert("cycleway:left:oneway", "no")?,
                None => {}
            }
            match right_cycle_lane {
                Some(Direction::Forward) => {
                    tags.checked_insert("cycleway:right:oneway", "yes")?;
                }
                Some(Direction::Backward) => {
                    tags.checked_insert("cycleway:right:oneway", "-1")?;
                }
                Some(Direction::Both) => tags.checked_insert("cycleway:right:oneway", "no")?,
                None => {}
            }
        }
    }
    // Bus Lanes
    {
        let left_bus_lane = lanes
            .iter()
            .take_while(|lane| !lane.is_motor())
            .find(|lane| lane.is_bus());
        let right_bus_lane = lanes
            .iter()
            .rev()
            .take_while(|lane| !lane.is_motor())
            .find(|lane| lane.is_bus());
        if left_bus_lane.is_none() && right_bus_lane.is_none() && lanes.iter().any(Lane::is_bus) {
            tags.checked_insert(
                "bus:lanes",
                lanes
                    .iter()
                    .map(|lane| if lane.is_bus() { "designated" } else { "" })
                    .collect::<Vec<_>>()
                    .as_slice()
                    .join("|"),
            )?;
        } else {
            let value = |lane: &Lane| -> &'static str {
                if oneway && lane.direction() == Some(Direction::Backward) {
                    "opposite_lane"
                } else {
                    "lane"
                }
            };
            match (left_bus_lane, right_bus_lane) {
                (None, None) => {}
                (Some(left), None) => tags.checked_insert("busway:left", value(left))?,
                (None, Some(right)) => tags.checked_insert("busway:right", value(right))?,
                (Some(_left), Some(_right)) => tags.checked_insert("busway:both", "lane")?,
            }
        }
    }

    let max_speed = {
        let max_speeds: Vec<Speed> = lanes
            .iter()
            .filter_map(|lane| match lane {
                Lane::Travel { max_speed, .. } => *max_speed,
                _ => None,
            })
            .collect();
        if let Some(max_speed) = max_speeds.first() {
            // Check if all are the same
            // See benches/benchmark_all_same.rs
            if max_speeds.windows(2).all(|w| w[0] == w[1]) {
                tags.checked_insert("maxspeed", max_speed.to_string())?;
                Some(*max_speed)
            } else {
                return Err(RoadMsg::Unimplemented {
                    description: Some("different max speeds per lane".to_owned()),
                    tags: None,
                }
                .into());
            }
        } else {
            None
        }
    };

    // Locale Specific Stuff
    if max_speed == Some(Speed::Kph(100.0)) && locale.country == Some(Country::the_netherlands()) {
        tags.checked_insert("motorroad", "yes")?;
    }

    // Check roundtrip!
    if config.check_roundtrip {
        let rountrip = tags_to_lanes(
            &tags,
            locale,
            &TagsToLanesConfig {
                error_on_warnings: true,
                ..TagsToLanesConfig::default()
            },
        )?;
        if lanes != rountrip.road.lanes {
            return Err(RoadError::RoundTrip);
        }
    }

    Ok(tags)
}
