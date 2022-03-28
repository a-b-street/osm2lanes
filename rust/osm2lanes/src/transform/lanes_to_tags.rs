use celes::Country;

use super::*;
use crate::road::{Lane, LaneDesignated, LaneDirection};
use crate::tag::{DuplicateKeyError, Tags, TagsWrite};
use crate::{Locale, Speed};

impl std::convert::From<DuplicateKeyError> for RoadError {
    fn from(e: DuplicateKeyError) -> Self {
        RoadError::Msg(RoadMsg::TagsDuplicateKey(e))
    }
}

#[non_exhaustive]
pub struct LanesToTagsConfig {
    pub check_roundtrip: bool,
}

impl LanesToTagsConfig {
    pub fn new(check_roundtrip: bool) -> Self {
        LanesToTagsConfig { check_roundtrip }
    }
}

impl Default for LanesToTagsConfig {
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

pub fn lanes_to_tags(lanes: &[Lane], locale: &Locale, config: &LanesToTagsConfig) -> TagsResult {
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
                    designated: LaneDesignated::Motor | LaneDesignated::Bus,
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
                direction: Some(LaneDirection::Forward),
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
                        designated: LaneDesignated::Motor | LaneDesignated::Bus,
                        direction: Some(LaneDirection::Forward),
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
                        designated: LaneDesignated::Motor | LaneDesignated::Bus,
                        direction: Some(LaneDirection::Backward),
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
                    designated: LaneDesignated::Motor,
                    direction: Some(LaneDirection::Both),
                    ..
                }
            )
        }) {
            tags.checked_insert("lanes:both_ways", "1")?;
            // TODO: add LHT support
            if lane_count >= 3 {
                tags.checked_insert("turn:lanes:both_ways", "left")?;
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
        let left_cycle_lane: Option<LaneDirection> = lanes
            .iter()
            .take_while(|lane| !lane.is_motor())
            .find(|lane| lane.is_bicycle())
            .and_then(|lane| lane.direction());
        let right_cycle_lane: Option<LaneDirection> = lanes
            .iter()
            .rev()
            .take_while(|lane| !lane.is_motor())
            .find(|lane| lane.is_bicycle())
            .and_then(|lane| lane.direction());
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
                && (left_cycle_lane.map_or(false, |direction| direction == LaneDirection::Backward)
                    || right_cycle_lane
                        .map_or(false, |direction| direction == LaneDirection::Backward))
            {
                tags.checked_insert("oneway:bicycle", "no")?;
            }
            // indicate cycling traffic direction relative to the direction the osm way is oriented
            // yes: same direction
            // -1: contraflow
            // no: bidirectional
            match left_cycle_lane {
                Some(LaneDirection::Forward) => {
                    tags.checked_insert("cycleway:left:oneway", "yes")?
                }
                Some(LaneDirection::Backward) => {
                    tags.checked_insert("cycleway:left:oneway", "-1")?
                }
                Some(LaneDirection::Both) => tags.checked_insert("cycleway:left:oneway", "no")?,
                None => {}
            }
            match right_cycle_lane {
                Some(LaneDirection::Forward) => {
                    tags.checked_insert("cycleway:right:oneway", "yes")?
                }
                Some(LaneDirection::Backward) => {
                    tags.checked_insert("cycleway:right:oneway", "-1")?
                }
                Some(LaneDirection::Both) => tags.checked_insert("cycleway:right:oneway", "no")?,
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
        if left_bus_lane.is_none()
            && right_bus_lane.is_none()
            && lanes.iter().any(|lane| lane.is_bus())
        {
            tags.checked_insert(
                "bus:lanes",
                lanes
                    .iter()
                    .map(|lane| if lane.is_bus() { "designated" } else { "" })
                    .collect::<Vec<_>>()
                    .as_slice()
                    .join("|"),
            )?
        } else {
            let value = |lane: &Lane| -> &'static str {
                if oneway && lane.direction() == Some(LaneDirection::Backward) {
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
