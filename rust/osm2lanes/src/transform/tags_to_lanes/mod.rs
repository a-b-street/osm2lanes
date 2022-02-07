use std::iter;

use crate::road::{Lane, LaneDesignated, LaneDirection, Marking, MarkingColor, MarkingStyle};
use crate::tags::{TagKey, Tags, TagsRead};
use crate::{DrivingSide, Locale};

mod bicycle;
use bicycle::bicycle;

mod bus;
use bus::bus;

mod foot_shoulder;
use foot_shoulder::foot_and_shoulder;

mod parking;
use parking::parking;

mod separator;
use separator::insert_separators;

mod non_motorized;
use non_motorized::non_motorized;

use super::*;

#[non_exhaustive]
pub struct TagsToLanesConfig {
    pub error_on_warnings: bool,
    pub include_separators: bool,
}

impl Default for TagsToLanesConfig {
    fn default() -> Self {
        Self {
            error_on_warnings: false,
            include_separators: true,
        }
    }
}

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
/// Warnings are produced for situations that maybe result in accurate lanes.
pub fn tags_to_lanes(tags: &Tags, locale: &Locale, config: &TagsToLanesConfig) -> LanesResult {
    let mut warnings = RoadWarnings::default();

    unsupported(tags, locale, &mut warnings)?;

    // Early return for non-motorized ways (pedestrian paths, cycle paths, etc.)
    if let Some(spec) = non_motorized(tags, locale)? {
        return Ok(spec);
    }

    let oneway = tags.is("oneway", "yes") || tags.is("junction", "roundabout");

    let (num_driving_fwd, num_driving_back) = driving_lane_directions(tags, locale, oneway);

    let driving_lane = if tags.is("access", "no")
        && (tags.is("bus", "yes") || tags.is("psv", "yes")) // West Seattle
        || tags
            .get("motor_vehicle:conditional")
            .map(|x| x.starts_with("no"))
            .unwrap_or(false)
            && tags.is("bus", "yes")
    // Example: 3rd Ave in downtown Seattle
    {
        LaneDesignated::Bus
    } else {
        LaneDesignated::Motor
    };

    // These are ordered from the road center, going outwards. Most of the members of fwd_side will
    // have Direction::Forward, but there can be exceptions with two-way cycletracks.
    let mut fwd_side: Vec<Lane> = iter::repeat_with(|| Lane::forward(driving_lane))
        .take(num_driving_fwd)
        .collect();
    let mut back_side: Vec<Lane> = iter::repeat_with(|| Lane::backward(driving_lane))
        .take(num_driving_back)
        .collect();
    // TODO Fix upstream. https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane
    if tags.is("lanes:both_ways", "1") || tags.is("centre_turn_lane", "yes") {
        fwd_side.insert(0, Lane::both(LaneDesignated::Motor));
    }

    bus(tags, locale, oneway, &mut fwd_side, &mut back_side)?;

    bicycle(
        tags,
        locale,
        oneway,
        &mut fwd_side,
        &mut back_side,
        &mut warnings,
    )?;

    if driving_lane == LaneDesignated::Motor {
        parking(tags, locale, oneway, &mut fwd_side, &mut back_side);
    }

    foot_and_shoulder(
        tags,
        locale,
        oneway,
        &mut fwd_side,
        &mut back_side,
        &mut warnings,
    )?;

    let lanes = assemble_ltr(fwd_side, back_side, locale.driving_side)?;

    let lanes = Lanes { lanes, warnings };

    let lanes = if config.include_separators {
        insert_separators(lanes)?
    } else {
        lanes
    };

    if config.error_on_warnings && !lanes.warnings.is_empty() {
        return Err(lanes.warnings.into());
    }

    Ok(lanes)
}

fn driving_lane_directions(tags: &Tags, _locale: &Locale, oneway: bool) -> (usize, usize) {
    let both_ways = if let Some(n) = tags
        .get("lanes:both_ways")
        .and_then(|num| num.parse::<usize>().ok())
    {
        n
    } else {
        0
    };
    let num_driving_fwd = if let Some(n) = tags
        .get("lanes:forward")
        .and_then(|num| num.parse::<usize>().ok())
    {
        n
    } else if let Some(n) = tags.get("lanes").and_then(|num| num.parse::<usize>().ok()) {
        let half = if oneway {
            n
        } else {
            // usize division rounded up
            (n + 1) / 2
        };
        half - both_ways
    } else if tags.is("lanes:bus", "2") {
        2
    } else {
        1
    };
    let num_driving_back = if let Some(n) = tags
        .get("lanes:backward")
        .and_then(|num| num.parse::<usize>().ok())
    {
        n
    } else if let Some(n) = tags.get("lanes").and_then(|num| num.parse::<usize>().ok()) {
        let base = n - num_driving_fwd;
        let half = if oneway {
            base
        } else {
            // lanes=1 but not oneway... what is this supposed to mean?
            base.max(1)
        };
        half - both_ways
    } else if tags.is("lanes:bus", "2") {
        if oneway {
            1
        } else {
            2
        }
    } else if oneway {
        0
    } else {
        1
    };
    (num_driving_fwd, num_driving_back)
}

fn assemble_ltr(
    mut fwd_side: Vec<Lane>,
    mut back_side: Vec<Lane>,
    driving_side: DrivingSide,
) -> Result<Vec<Lane>, RoadError> {
    Ok(match driving_side {
        DrivingSide::Right => {
            back_side.reverse();
            back_side.extend(fwd_side);
            back_side
        }
        DrivingSide::Left => {
            fwd_side.reverse();
            fwd_side.extend(back_side);
            fwd_side
        }
    })
}

pub fn unsupported(tags: &Tags, _locale: &Locale, warnings: &mut RoadWarnings) -> ModeResult {
    if tags.is("highway", "bus_guideway") {
        return Err(RoadMsg::unimplemented_tag("highway", "bus_guideway").into());
    }

    if tags.is("highway", "construction") {
        return Err(RoadMsg::unimplemented_tag("highway", "construction").into());
    }

    let tag_tree = tags.tree();
    if tag_tree
        .get("lanes")
        .map_or(false, |val| val.tree().is_some())
    {
        warnings.push(RoadMsg::Unimplemented {
            description: Some("lanes=*".to_owned()),
            // TODO, TagTree should support subset
            tags: Some(tags.subset(&["lanes"])),
        });
    }

    // https://wiki.openstreetmap.org/wiki/Key:access#Transport_mode_restrictions
    const ACCESS_KEYS: [&str; 43] = [
        "access",
        "dog",
        "ski",
        "inline_skates",
        "horse",
        "vehicle",
        "bicycle",
        "electric_bicycle",
        "carriage",
        "hand_cart",
        "quadracycle",
        "trailer",
        "caravan",
        "motor_vehicle",
        "motorcycle",
        "moped",
        "mofa",
        "motorcar",
        "motorhome",
        "tourist_bus",
        "coach",
        "goods",
        "hgv",
        "hgv_articulated",
        "bdouble",
        "agricultural",
        "golf_cart",
        "atv",
        "snowmobile",
        "psv",
        "bus",
        "taxi",
        "minibus",
        "share_taxi",
        "hov",
        "car_sharing",
        "emergency",
        "hazmat",
        "disabled",
        "roadtrain",
        "hgv_caravan",
        "lhv",
        "tank",
    ];
    if ACCESS_KEYS
        .iter()
        .any(|k| tags.get(TagKey::from(k)).is_some())
    {
        warnings.push(RoadMsg::Unimplemented {
            description: Some("access".to_owned()),
            // TODO, TagTree should support subset
            tags: Some(tags.subset(&ACCESS_KEYS)),
        });
    }

    if tags.is("oneway", "reversible") {
        // TODO reversible roads should be handled differently
        return Err(RoadMsg::unimplemented_tag("oneway", "reversible").into());
    }

    Ok(())
}
