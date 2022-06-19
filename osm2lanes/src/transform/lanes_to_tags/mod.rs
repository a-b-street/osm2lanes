#![allow(clippy::module_name_repetitions)] // TODO: fix upstream

use celes::Country;
use osm_tag_schemes::Access;
use osm_tags::Tags;

use super::{tags_to_lanes, TagsToLanesConfig};
use crate::locale::{DrivingSide, Locale};
use crate::metric::Speed;
use crate::road::{AccessByType, Color, Designated, Direction, Lane, Marking, Road};

mod error;
pub use error::LanesToTagsMsg;

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

    fn access(&self) -> Option<&AccessByType> {
        match self {
            Self::Travel { access, .. } => access.as_ref(),
            _ => None,
        }
    }
}

impl Road {
    /// Find the lanes going forward on the main continuous road surface
    fn travel_lanes<'this>(
        &'this self,
        direction: Direction,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &Lane> + 'this> {
        if let Direction::Both = direction {
            unreachable!();
        }
        let skip_while = move |lane: &&Lane| {
            !matches!(
                lane,
                Lane::Travel {
                    designated: Designated::Motor,
                    direction: Some(travel_direction),
                    ..
                } if direction == *travel_direction
            )
        };
        let take_while = move |lane: &&Lane| {
            matches!(
                lane,
                Lane::Travel {
                    direction: Some(travel_direction),
                    ..
                } if direction == *travel_direction
            )
        };
        let left_to_right = match (direction, locale.driving_side) {
            (Direction::Forward, DrivingSide::Right) => true,
            (Direction::Forward, DrivingSide::Left) => false,
            (Direction::Backward, DrivingSide::Right) => false,
            (Direction::Backward, DrivingSide::Left) => true,
            (Direction::Both, _) => unreachable!(),
        };
        if left_to_right {
            Box::new(
                self.lanes
                    .iter()
                    .skip_while(skip_while)
                    .take_while(take_while),
            )
        } else {
            Box::new(
                self.lanes
                    .iter()
                    .rev()
                    .skip_while(skip_while)
                    .take_while(take_while),
            )
        }
    }
    /// Find the lanes going forward on the main continuous road surface
    fn forward_travel_lanes<'this>(
        &'this self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &Lane> + 'this> {
        self.travel_lanes(Direction::Forward, locale)
    }
    /// Find the lanes going backward on the main continuous road surface
    fn backward_travel_lanes<'this>(
        &'this self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &Lane> + 'this> {
        self.travel_lanes(Direction::Backward, locale)
    }
}

/// Convert Lanes back to Tags
///
/// # Errors
///
/// Any of:
/// - internal error
/// - unimplemented or unsupported functionality
/// - the OSM tag spec cannot represent the lanes
pub fn lanes_to_tags(
    road: &Road,
    locale: &Locale,
    config: &Config,
) -> Result<Tags, LanesToTagsMsg> {
    let mut tags = Tags::default();

    if !road
        .lanes
        .iter()
        .any(|lane| lane.is_motor() || lane.is_bus())
    {
        tags.checked_insert("highway", "path")?;
        return Ok(tags);
    }

    tags.checked_insert("highway", road.highway.r#type().to_string())?;
    if road.highway.is_construction() {
        return Err(LanesToTagsMsg::unimplemented("construction=*"));
    }
    if road.highway.is_proposed() {
        return Err(LanesToTagsMsg::unimplemented("proposed=*"));
    }

    let lanes = &road.lanes;

    let lane_count = set_lanes(lanes, &mut tags)?;
    let oneway = set_oneway(lanes, &mut tags, locale, lane_count)?;

    set_shoulder(lanes, &mut tags)?;
    set_pedestrian(lanes, &mut tags)?;
    set_parking(lanes, &mut tags)?;
    set_cycleway(lanes, &mut tags, oneway, locale)?;
    set_busway(lanes, &mut tags, oneway)?;

    let max_speed = get_max_speed(lanes, &mut tags)?;

    locale_additions(max_speed, locale, &mut tags)?;

    check_roundtrip(config, &tags, locale, lanes)?;

    Ok(tags)
}

fn set_lanes(lanes: &[Lane], tags: &mut Tags) -> Result<usize, LanesToTagsMsg> {
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
    Ok(lane_count)
}

/// Returns oneway
fn set_oneway(
    lanes: &[Lane],
    tags: &mut Tags,
    locale: &Locale,
    lane_count: usize,
) -> Result<bool, LanesToTagsMsg> {
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
        Ok(true)
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
        Ok(false)
    }
}

fn set_shoulder(lanes: &[Lane], tags: &mut Tags) -> Result<(), LanesToTagsMsg> {
    match (
        lanes.first().unwrap().is_shoulder(),
        lanes.last().unwrap().is_shoulder(),
    ) {
        (false, false) => {
            // TODO do we want to always be explicit about this?
            tags.checked_insert("shoulder", "no")?;
        },
        (true, false) => {
            tags.checked_insert("shoulder", "left")?;
        },
        (false, true) => {
            tags.checked_insert("shoulder", "right")?;
        },
        (true, true) => tags.checked_insert("shoulder", "both")?,
    }
    Ok(())
}

fn set_pedestrian(lanes: &[Lane], tags: &mut Tags) -> Result<(), LanesToTagsMsg> {
    match (
        lanes.first().unwrap().is_foot(),
        lanes.last().unwrap().is_foot(),
    ) {
        (false, false) => {
            // TODO do we want to always be explicit about this?
            tags.checked_insert("sidewalk", "no")?;
        },
        (true, false) => tags.checked_insert("sidewalk", "left")?,
        (false, true) => tags.checked_insert("sidewalk", "right")?,
        (true, true) => tags.checked_insert("sidewalk", "both")?,
    }
    Ok(())
}

fn set_parking(lanes: &[Lane], tags: &mut Tags) -> Result<(), LanesToTagsMsg> {
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
        (false, false) => {},
        (true, false) => tags.checked_insert("parking:lane:left", "parallel")?,
        (false, true) => tags.checked_insert("parking:lane:right", "parallel")?,
        (true, true) => tags.checked_insert("parking:lane:both", "parallel")?,
    }

    if let Some(Lane::Separator { markings, .. }) = lanes.first() {
        if let Some(Marking {
            color: Some(Color::Red),
            ..
        }) = markings.as_ref().and_then(|m| m.first())
        {
            tags.checked_insert("parking:condition:both", "no_stopping")?;
        }
    }

    Ok(())
}

fn set_cycleway(
    lanes: &[Lane],
    tags: &mut Tags,
    oneway: bool,
    locale: &Locale,
) -> Result<(), LanesToTagsMsg> {
    let left_cycle_lane: Option<&Lane> = lanes
        .iter()
        .take_while(|lane| !lane.is_motor())
        .find(|lane| lane.is_bicycle());
    let right_cycle_lane: Option<&Lane> = lanes
        .iter()
        .rev()
        .take_while(|lane| !lane.is_motor())
        .find(|lane| lane.is_bicycle());
    match (left_cycle_lane.is_some(), right_cycle_lane.is_some()) {
        (false, false) => {},
        (true, false) => tags.checked_insert("cycleway:left", "lane")?,
        (false, true) => tags.checked_insert("cycleway:right", "lane")?,
        (true, true) => tags.checked_insert("cycleway:both", "lane")?,
    }

    // if the way has oneway=yes and you are allowed to cycle against that oneway flow
    // also add oneway:bicycle=no to make it easier
    // for bicycle routers to see that the way can be used in two directions.
    if oneway
        && (left_cycle_lane
            .and_then(Lane::direction)
            .map_or(false, |direction| direction == Direction::Backward)
            || right_cycle_lane
                .and_then(Lane::direction)
                .map_or(false, |direction| direction == Direction::Backward))
    {
        tags.checked_insert("oneway:bicycle", "no")?;
    }

    // indicate cycling traffic direction relative to the direction the osm way is oriented
    // yes: same direction
    // -1: contraflow
    // no: bidirectional
    match left_cycle_lane.and_then(Lane::direction) {
        Some(Direction::Forward) => {
            tags.checked_insert("cycleway:left:oneway", "yes")?;
        },
        Some(Direction::Backward) => {
            tags.checked_insert("cycleway:left:oneway", "-1")?;
        },
        Some(Direction::Both) => tags.checked_insert("cycleway:left:oneway", "no")?,
        None => {},
    }
    match right_cycle_lane.and_then(Lane::direction) {
        Some(Direction::Forward) => {
            tags.checked_insert("cycleway:right:oneway", "yes")?;
        },
        Some(Direction::Backward) => {
            tags.checked_insert("cycleway:right:oneway", "-1")?;
        },
        Some(Direction::Both) => tags.checked_insert("cycleway:right:oneway", "no")?,
        None => {},
    }

    if let Some(Lane::Travel {
        width: Some(width), ..
    }) = left_cycle_lane
    {
        tags.checked_insert("cycleway:left:width", width.val().to_string())?;
    }
    if let Some(Lane::Travel {
        width: Some(width), ..
    }) = right_cycle_lane
    {
        tags.checked_insert("cycleway:right:width", width.val().to_string())?;
    }

    // Handle shared lanes
    //if lanes.forward_inside() // TODO: this needs to exist...
    if lanes.len() == 1 {
        let lane = match locale.driving_side {
            DrivingSide::Right => lanes.last(),
            DrivingSide::Left => lanes.first(),
        };
        if let Some(bicycle) = lane.and_then(Lane::access).and_then(|a| a.bicycle.as_ref()) {
            if oneway && bicycle.access == Access::Yes && bicycle.direction == Some(Direction::Both)
            {
                tags.checked_insert("cycleway", "opposite")?;
            }
        }
    }

    Ok(())
}

fn set_busway(lanes: &[Lane], tags: &mut Tags, oneway: bool) -> Result<(), LanesToTagsMsg> {
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
            (None, None) => {},
            (Some(left), None) => tags.checked_insert("busway:left", value(left))?,
            (None, Some(right)) => tags.checked_insert("busway:right", value(right))?,
            (Some(_left), Some(_right)) => tags.checked_insert("busway:both", "lane")?,
        }
    }
    Ok(())
}

fn get_max_speed(lanes: &[Lane], tags: &mut Tags) -> Result<Option<Speed>, LanesToTagsMsg> {
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
        if max_speeds.windows(2).all(|w| {
            let speeds: &[Speed; 2] = w.try_into().unwrap();
            speeds[0] == speeds[1]
        }) {
            tags.checked_insert("maxspeed", max_speed.to_string())?;
            Ok(Some(*max_speed))
        } else {
            Err(LanesToTagsMsg::unimplemented(
                "different max speeds per lane",
            ))
        }
    } else {
        Ok(None)
    }
}

fn locale_additions(
    max_speed: Option<Speed>,
    locale: &Locale,
    tags: &mut Tags,
) -> Result<(), LanesToTagsMsg> {
    if max_speed == Some(Speed::Kph(100.0)) && locale.country == Some(Country::the_netherlands()) {
        tags.checked_insert("motorroad", "yes")?;
    }
    Ok(())
}

fn check_roundtrip(
    config: &Config,
    tags: &Tags,
    locale: &Locale,
    lanes: &[Lane],
) -> Result<(), LanesToTagsMsg> {
    if config.check_roundtrip {
        let rountrip = tags_to_lanes(
            tags,
            locale,
            &TagsToLanesConfig {
                error_on_warnings: true,
                ..TagsToLanesConfig::default()
            },
        )?;
        if lanes != rountrip.road.lanes {
            return Err(LanesToTagsMsg::roundtrip());
        }
    }
    Ok(())
}
