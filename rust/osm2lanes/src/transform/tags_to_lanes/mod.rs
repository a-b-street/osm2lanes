use std::iter;

use crate::tags::{TagKey, Tags, TagsRead};
use crate::{DrivingSide, Lane, LaneDesignated, LaneDirection, Locale};

mod bicycle;
use bicycle::bicycle;
mod bus;
use bus::bus;
mod foot_shoulder;
use foot_shoulder::foot_and_shoulder;

use super::*;

#[derive(Default)]
pub struct TagsToLanesConfig {
    pub error_on_warnings: bool,
}

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
/// Warnings are produced for situations that maybe result in accurate lanes.
pub fn tags_to_lanes(tags: &Tags, locale: &Locale, config: &TagsToLanesConfig) -> LanesResult {
    let mut warnings = RoadWarnings::default();

    if let Some(spec) = non_motorized(tags, locale) {
        return spec;
    }

    // TODO Reversible roads should be handled differently?
    let oneway = tags.is_any("oneway", &["yes", "reversible"]) || tags.is("junction", "roundabout");

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
    // } else if tags.is("access", "no") || tags.is("highway", "construction") {
    //     LaneType::Construction
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

    // if driving_lane == LaneType::Construction {
    //     return Ok(Lanes {
    //         lanes: assemble_ltr(fwd_side, back_side, cfg.driving_side),
    //         warnings: LaneSpecWarnings::default(),
    //     });
    // }

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

    if config.error_on_warnings && !warnings.is_empty() {
        return Err(warnings.into());
    }

    Ok(Lanes {
        lanes: assemble_ltr(fwd_side, back_side, locale.driving_side),
        warnings,
    })
}

// Handle non motorized ways
fn non_motorized(tags: &Tags, locale: &Locale) -> Option<LanesResult> {
    if !tags.is_any(
        HIGHWAY,
        &[
            "cycleway",
            "footway",
            "path",
            "pedestrian",
            "steps",
            "track",
        ],
    ) {
        log::trace!("motorized");
        return None;
    }
    // Easy special cases first.
    if tags.is(HIGHWAY, "steps") {
        return Some(Ok(Lanes {
            lanes: vec![Lane::foot()],
            warnings: RoadWarnings(vec![RoadMsg::Other {
                description: "highway is steps, but lane is only a sidewalk".to_owned(),
                tags: tags.subset(&[HIGHWAY]),
            }]),
        }));
    }

    // Eventually, we should have some kind of special LaneType for shared walking/cycling paths of
    // different kinds. Until then, model by making bike lanes and a shoulder for walking.

    // If it just allows foot traffic, simply make it a sidewalk. For most of the above highway
    // types, assume bikes are allowed, except for footways, where they must be explicitly
    // allowed.
    if tags.is("bicycle", "no")
        || (tags.is(HIGHWAY, "footway") && !tags.is_any("bicycle", &["designated", "yes"]))
    {
        return Some(Ok(Lanes {
            lanes: vec![Lane::foot()],
            warnings: RoadWarnings::default(),
        }));
    }
    // Otherwise, there'll always be a bike lane.

    let mut forward_side = vec![Lane::forward(LaneDesignated::Bicycle)];
    let mut backward_side = if tags.is("oneway", "yes") {
        vec![]
    } else {
        vec![Lane::backward(LaneDesignated::Bicycle)]
    };

    if !tags.is("foot", "no") {
        forward_side.push(Lane::Shoulder);
        if !backward_side.is_empty() {
            backward_side.push(Lane::Shoulder);
        }
    }
    Some(Ok(Lanes {
        lanes: assemble_ltr(forward_side, backward_side, locale.driving_side),
        warnings: RoadWarnings::default(),
    }))
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

fn parking(
    tags: &Tags,
    _locale: &Locale,
    _oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) {
    let has_parking = vec!["parallel", "diagonal", "perpendicular"];
    let parking_lane_fwd = tags.is_any("parking:lane:right", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    let parking_lane_back = tags.is_any("parking:lane:left", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    if parking_lane_fwd {
        forward_side.push(Lane::parking(LaneDirection::Forward));
    }
    if parking_lane_back {
        backward_side.push(Lane::parking(LaneDirection::Backward));
    }
}

fn assemble_ltr(
    mut fwd_side: Vec<Lane>,
    mut back_side: Vec<Lane>,
    driving_side: DrivingSide,
) -> Vec<Lane> {
    match driving_side {
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
    }
}
