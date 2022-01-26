use std::iter;

use crate::tags::{TagKey, Tags, TagsRead};
use crate::{DrivingSide, Lane, LaneDesignated, LaneDirection, Locale, Road, RoadError};

use super::*;

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
/// Warnings generate and error.
/// To ignore warnings, use tags_to_lanes_with_warnings and ignore them explicitly.
pub fn tags_to_lanes(tags: &Tags, locale: &Locale) -> RoadResult {
    let Lanes { lanes, warnings } = tags_to_lanes_with_warnings(tags, locale)?;
    if !warnings.0.is_empty() {
        return Err(format!("{} warnings found", warnings.0.len()).into());
    }
    Ok(Road { lanes })
}

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
/// Warnings are produced for any ambiguity.
pub fn tags_to_lanes_with_warnings(tags: &Tags, locale: &Locale) -> LanesResult {
    let mut warnings = LaneWarnings::default();

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
            warnings: LaneWarnings(vec![LaneSpecWarning {
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
            warnings: LaneWarnings::default(),
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
        warnings: LaneWarnings::default(),
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

fn bus(
    tags: &Tags,
    locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) -> ModeResult {
    // https://wiki.openstreetmap.org/wiki/Bus_lanes
    // 3 schemes, for simplicity we only allow one at a time
    let tag_tree = tags.tree();

    match (
        tag_tree.get("busway").is_some(),
        tag_tree
            .get("lanes:psv")
            .or_else(|| tag_tree.get("lanes:bus"))
            .is_some(),
        tag_tree
            .get("bus:lanes")
            .or_else(|| tag_tree.get("psv:lanes"))
            .is_some(),
    ) {
        (false, false, false) => {}
        (true, _, false) => bus_busway(tags, locale, oneway, forward_side, backward_side)?,
        (false, true, false) => bus_bus_lanes(tags, locale, oneway, forward_side, backward_side)?,
        (false, false, true) => bus_lanes_bus(tags, locale, oneway, forward_side, backward_side)?,
        _ => return Err("more than one bus lanes scheme used".into()),
    }

    Ok(())
}

fn bus_busway(
    tags: &Tags,
    locale: &Locale,
    _oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) -> ModeResult {
    const BUSWAY: TagKey = TagKey::from("busway");
    if tags.is(BUSWAY, "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::from("no forward lanes for busway"))?
            .set_bus()?;
        if !tags.is("oneway", "yes") && !tags.is("oneway:bus", "yes") {
            backward_side
                .last_mut()
                .ok_or_else(|| RoadError::from("no backward lanes for busway"))?
                .set_bus()?;
        }
    }
    if tags.is(BUSWAY, "opposite_lane") {
        backward_side
            .last_mut()
            .ok_or_else(|| RoadError::from("no backward lanes for busway"))?
            .set_bus()?;
    }
    if tags.is(BUSWAY + "both", "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::from("no forward lanes for busway"))?
            .set_bus()?;
        backward_side
            .last_mut()
            .ok_or_else(|| RoadError::from("no backward lanes for busway"))?
            .set_bus()?;
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            return Err("busway:both=lane is ambiguous for oneway roads".into());
        }
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::from("no forward lanes for busway"))?
            .set_bus()?;
    }
    if tags.is(BUSWAY + locale.driving_side.opposite().tag(), "lane") {
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            forward_side
                .first_mut()
                .ok_or_else(|| RoadError::from("no forward lanes for busway"))?
                .set_bus()?;
        } else {
            return Err("busway:BACKWARD=lane is ambiguous for bidirectional roads".into());
        }
    }
    Ok(())
}

fn bus_lanes_bus(
    _tags: &Tags,
    _locale: &Locale,
    _oneway: bool,
    _forward_side: &mut Vec<Lane>,
    _backward_side: &mut Vec<Lane>,
) -> ModeResult {
    Ok(())
}

fn bus_bus_lanes(
    tags: &Tags,
    _locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) -> ModeResult {
    let fwd_bus_spec = if let Some(s) = tags.get("bus:lanes:forward") {
        s
    } else if let Some(s) = tags.get("psv:lanes:forward") {
        s
    } else if oneway {
        if let Some(s) = tags.get("bus:lanes") {
            s
        } else if let Some(s) = tags.get("psv:lanes") {
            s
        } else {
            ""
        }
    } else {
        ""
    };
    if !fwd_bus_spec.is_empty() {
        let parts: Vec<&str> = fwd_bus_spec.split('|').collect();
        let offset = if let Lane::Travel {
            direction: Some(LaneDirection::Both),
            ..
        } = forward_side[0]
        {
            1
        } else {
            0
        };
        if parts.len() == forward_side.len() - offset {
            for (idx, part) in parts.into_iter().enumerate() {
                if part == "designated" {
                    let direction =
                        if let Lane::Travel { direction, .. } = forward_side[idx + offset] {
                            direction
                        } else {
                            unreachable!()
                        };
                    forward_side[idx + offset] = Lane::Travel {
                        direction,
                        designated: LaneDesignated::Bus,
                    };
                }
            }
        }
    }
    if let Some(spec) = tags
        .get("bus:lanes:backward")
        .or_else(|| tags.get("psv:lanes:backward"))
    {
        let parts: Vec<&str> = spec.split('|').collect();
        if parts.len() == backward_side.len() {
            for (idx, part) in parts.into_iter().enumerate() {
                if part == "designated" {
                    let direction = if let Lane::Travel { direction, .. } = forward_side[idx] {
                        direction
                    } else {
                        unreachable!()
                    };
                    backward_side[idx] = Lane::Travel {
                        direction,
                        designated: LaneDesignated::Bus,
                    };
                }
            }
        }
    }

    Ok(())
}

fn bicycle(
    tags: &Tags,
    locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
    warnings: &mut LaneWarnings,
) -> ModeResult {
    impl Tags {
        fn is_cycleway(&self, side: Option<WaySide>) -> bool {
            if let Some(side) = side {
                self.is_any(CYCLEWAY + side.as_str(), &["lane", "track"])
            } else {
                self.is_any(CYCLEWAY, &["lane", "track"])
            }
        }
    }

    if tags.is_cycleway(None) {
        if tags.is_cycleway(Some(WaySide::Both))
            || tags.is_cycleway(Some(WaySide::Right))
            || tags.is_cycleway(Some(WaySide::Left))
        {
            return Err("cycleway=* not supported with any cycleway:* values".into());
        }
        forward_side.push(Lane::forward(LaneDesignated::Bicycle));
        if oneway {
            if !backward_side.is_empty() {
                // TODO safety check to be checked
                warnings.0.push(LaneSpecWarning {
                    description: "oneway has backwards lanes when adding cycleways".to_owned(),
                    tags: tags.subset(&["oneway", "cycleway"]),
                })
            }
        } else {
            backward_side.push(Lane::backward(LaneDesignated::Bicycle));
        }
    } else if tags.is_cycleway(Some(WaySide::Both)) {
        forward_side.push(Lane::forward(LaneDesignated::Bicycle));
        backward_side.push(Lane::backward(LaneDesignated::Bicycle));
    } else {
        // cycleway=opposite_lane
        if tags.is(CYCLEWAY, "opposite_lane") {
            warnings.0.push(LaneSpecWarning {
                description: "cycleway=opposite_lane deprecated".to_owned(),
                tags: tags.subset(&[CYCLEWAY]),
            });
            backward_side.push(Lane::backward(LaneDesignated::Bicycle));
        }
        // cycleway:FORWARD=*
        if tags.is_cycleway(Some(locale.driving_side.into())) {
            if tags.is(CYCLEWAY + locale.driving_side.tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
            {
                forward_side.push(Lane::both(LaneDesignated::Bicycle));
            } else {
                forward_side.push(Lane::forward(LaneDesignated::Bicycle));
            }
        }
        // cycleway:FORWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            warnings.0.push(LaneSpecWarning {
                description: "cycleway:FORWARD=opposite_lane deprecated".to_owned(),
                tags: tags.subset(&[CYCLEWAY]), // TODO make side specific
            });
            forward_side.push(Lane::backward(LaneDesignated::Bicycle));
        }
        // cycleway:BACKWARD=*
        if tags.is_cycleway(Some(locale.driving_side.opposite().into())) {
            if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "no",
            ) || tags.is("oneway:bicycle", "no")
            {
                backward_side.push(Lane::both(LaneDesignated::Bicycle));
            } else if oneway {
                // A oneway road with a cycleway on the wrong side
                forward_side.insert(0, Lane::forward(LaneDesignated::Bicycle));
            } else {
                // A contraflow bicycle lane
                backward_side.push(Lane::backward(LaneDesignated::Bicycle));
            }
        }
        // cycleway:BACKWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.opposite().tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            return Err("cycleway:BACKWARD=opposite_lane unsupported".into());
        }
    }
    Ok(())
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

fn foot_and_shoulder(
    tags: &Tags,
    locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
    warnings: &mut LaneWarnings,
) -> ModeResult {
    // https://wiki.openstreetmap.org/wiki/Key:sidewalk
    // This first step processes tags by the OSM spec.
    // No can be implied, e.g. we assume that sidewalk:left=yes implies sidewalk:right=no
    // None is when information may be incomplete and should be inferred,
    // e.g. when sidewalk=* is missing altogether,
    // but this may later become a No when combined with data from shoulder=*
    // We catch any tag combinations that violate the OSM spec
    enum Sidewalk {
        None,
        No,
        Yes,
        Separate,
    }
    let sidewalk: (Sidewalk, Sidewalk) = match (
        tags.get(SIDEWALK),
        tags.get(SIDEWALK + locale.driving_side.tag()),
        tags.get(SIDEWALK + locale.driving_side.opposite().tag()),
    ) {
        (None, None, None) => (Sidewalk::None, Sidewalk::None),
        (Some("none"), None, None) => return Err("sidewalk=none is deprecated".into()),
        (Some("no"), None, None) => (Sidewalk::No, Sidewalk::No),
        (Some("yes"), None, None) => {
            warnings.0.push(LaneSpecWarning {
                description: "sidewalk=yes is ambiguous".to_owned(),
                tags: tags.subset(&[SIDEWALK]),
            });
            (Sidewalk::Yes, Sidewalk::Yes)
        }
        (Some("both"), None, None) => (Sidewalk::Yes, Sidewalk::Yes),
        (None, Some("yes"), Some("yes")) => (Sidewalk::Yes, Sidewalk::Yes),
        (Some(s), None, None) if s == locale.driving_side.tag().as_str() => {
            (Sidewalk::Yes, Sidewalk::No)
        }
        (None, Some("yes"), None | Some("no")) => (Sidewalk::Yes, Sidewalk::No),
        (Some(s), None, None) if s == locale.driving_side.opposite().tag().as_str() => {
            (Sidewalk::No, Sidewalk::Yes)
        }
        (None, None | Some("no"), Some("yes")) => (Sidewalk::No, Sidewalk::Yes),
        (Some("separate"), None, None) => (Sidewalk::Separate, Sidewalk::Separate),
        (None, Some("separate"), None) => (Sidewalk::Separate, Sidewalk::No),
        (None, None, Some("separate")) => (Sidewalk::No, Sidewalk::Separate),
        // TODO: generate the rest of these automatically
        (None, Some(forward), None) => {
            return Err(format!(
                "sidewalk:{}={} is unsupported",
                locale.driving_side.tag().as_str(),
                forward,
            )
            .into())
        }
        (None, None, Some(backward)) => {
            return Err(format!(
                "sidewalk:{}={} is unsupported",
                locale.driving_side.opposite().tag().as_str(),
                backward,
            )
            .into())
        }
        (Some(s), Some(forward), None) => {
            return Err(format!(
                "sidewalk={} and sidewalk:{}={} unsupported",
                s,
                locale.driving_side.tag().as_str(),
                forward,
            )
            .into())
        }
        (Some(s), None, Some(backward)) => {
            return Err(format!(
                "sidewalk={} and sidewalk:{}={} unsupported",
                s,
                locale.driving_side.opposite().tag().as_str(),
                backward,
            )
            .into())
        }
        (Some(s), None, None) => return Err(format!("sidewalk={} unknown", s).into()),
        (None, Some(forward), Some(backward)) => {
            return Err(format!(
                "sidewalk:{}={} and sidewalk:{}={} unknown",
                locale.driving_side.tag().as_str(),
                forward,
                locale.driving_side.opposite().tag().as_str(),
                backward
            )
            .into())
        }
        (Some(s), Some(forward), Some(backward)) => {
            return Err(format!(
                "sidewalk={} and sidewalk:{}={} and sidewalk:{}={} unknown",
                s,
                locale.driving_side.tag().as_str(),
                forward,
                locale.driving_side.opposite().tag().as_str(),
                backward
            )
            .into())
        }
    };

    // https://wiki.openstreetmap.org/wiki/Key:shoulder
    enum Shoulder {
        None,
        Yes,
        No,
    }
    let shoulder: (Shoulder, Shoulder) = match tags.get(SHOULDER) {
        None => (Shoulder::None, Shoulder::None),
        Some("no") => (Shoulder::No, Shoulder::No),
        Some("yes") => (Shoulder::Yes, Shoulder::Yes),
        Some("both") => (Shoulder::Yes, Shoulder::Yes),
        Some(s) if s == locale.driving_side.tag().as_str() => (Shoulder::Yes, Shoulder::No),
        Some(s) if s == locale.driving_side.opposite().tag().as_str() => {
            (Shoulder::No, Shoulder::Yes)
        }
        Some(s) => return Err(format!("Unknown shoulder={}", s).into()),
    };

    fn add(
        (sidewalk, shoulder): (Sidewalk, Shoulder),
        side: &mut Vec<Lane>,
        oneway: bool,
        forward: bool,
    ) -> ModeResult {
        match (sidewalk, shoulder) {
            (Sidewalk::No | Sidewalk::None, Shoulder::None) => {
                // We assume a shoulder if there is no bike lane.
                // This assumes bicycle lanes are just glorified shoulders...
                let has_bicycle_lane = side.last().map_or(false, |lane| lane.is_bicycle());

                if !has_bicycle_lane && (forward || !oneway) {
                    side.push(Lane::Shoulder)
                }
            }
            (Sidewalk::No | Sidewalk::None, Shoulder::No) => {}
            (Sidewalk::Yes, Shoulder::No | Shoulder::None) => side.push(Lane::foot()),
            (Sidewalk::No | Sidewalk::None, Shoulder::Yes) => side.push(Lane::Shoulder),
            (Sidewalk::Yes, Shoulder::Yes) => {
                return Err("shoulder=* and sidewalk=* on same side".into())
            }
            (Sidewalk::Separate, _) => return Err("sidewalk=separate not supported".into()),
        }
        Ok(())
    }

    add((sidewalk.0, shoulder.0), forward_side, oneway, true)?;
    add((sidewalk.1, shoulder.1), backward_side, oneway, false)?;

    Ok(())
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
