use super::*;
use crate::road::{Lane, LaneDesignated, LaneDirection};
use crate::tags::{DuplicateKeyError, Tags, TagsWrite};
use crate::Locale;

impl std::convert::From<DuplicateKeyError> for RoadError {
    fn from(e: DuplicateKeyError) -> Self {
        RoadError::Msg(RoadMsg::TagsDuplicateKey(e))
    }
}

#[non_exhaustive]
pub struct LanesToTagsConfig {
    pub check_roundtrip: bool,
}

impl Default for LanesToTagsConfig {
    fn default() -> Self {
        Self {
            check_roundtrip: true,
        }
    }
}

pub fn lanes_to_tags(lanes: &[Lane], locale: &Locale, config: &LanesToTagsConfig) -> TagsResult {
    let mut tags = Tags::default();
    let mut oneway = false;
    tags.checked_insert("highway", "yes")?; // TODO, what?
    {
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
    }
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
        tags.insert("oneway", "yes");
        oneway = true;
    }
    // Shoulder
    match (
        lanes.first().unwrap() == &Lane::Shoulder,
        lanes.last().unwrap() == &Lane::Shoulder,
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
        match (left_bus_lane.is_some(), right_bus_lane.is_some()) {
            (false, false) => {}
            (true, false) => tags.checked_insert("busway:left", "lane")?,
            (false, true) => tags.checked_insert("busway:right", "lane")?,
            (true, true) => tags.checked_insert("busway:both", "lane")?,
        }
    }

    if lanes.iter().any(|lane| {
        matches!(
            lane,
            Lane::Travel {
                designated: LaneDesignated::Motor,
                direction: Some(LaneDirection::Both),
            }
        )
    }) {
        tags.checked_insert("lanes:both_ways", "1")?;
        // TODO: add LHT support
        tags.checked_insert("turn:lanes:both_ways", "left")?;
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
        if lanes != rountrip.lanes {
            return Err(RoadError::RoundTrip);
        }
    }

    Ok(tags)
}
