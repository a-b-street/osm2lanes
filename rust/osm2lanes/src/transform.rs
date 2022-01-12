use std::iter;

use serde::{Deserialize, Serialize};

use crate::{BufferType, Config, Direction, DrivingSide, LaneSpec, LaneType, Tags};

const HIGHWAY: &str = "highway";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneSpecError(String);

#[derive(Default)]
pub struct LaneSpecWarnings(Vec<LaneSpecWarning>);

pub struct LaneSpecWarning {
    _description: String,
    // Tags relevant to triggering the warning
    _tags: Tags,
}

/// From an OpenStreetMap way's tags, determine the lanes along the road from left to right.
pub fn get_lane_specs_ltr_with_warnings(
    tags: Tags,
    cfg: &Config,
) -> Result<(Vec<LaneSpec>, LaneSpecWarnings), LaneSpecError> {
    let fwd = |lane_type: LaneType| LaneSpec {
        lane_type,
        direction: Direction::Forward,
    };
    let back = |lane_type: LaneType| LaneSpec {
        lane_type,
        direction: Direction::Backward,
    };

    // Easy special cases first.
    if tags.is(HIGHWAY, "steps") {
        return Ok((vec![fwd(LaneType::Sidewalk)], LaneSpecWarnings::default()));
    }
    // Eventually, we should have some kind of special LaneType for shared walking/cycling paths of
    // different kinds. Until then, model by making bike lanes and a shoulder for walking.
    if tags.is_any(
        HIGHWAY,
        vec!["cycleway", "footway", "path", "pedestrian", "track"],
    ) {
        // If it just allows foot traffic, simply make it a sidewalk. For most of the above highway
        // types, assume bikes are allowed, except for footways, where they must be explicitly
        // allowed.
        if tags.is("bicycle", "no")
            || (tags.is(HIGHWAY, "footway")
                && !tags.is_any("bicycle", vec!["designated", "yes", "dismount"]))
        {
            return Ok((vec![fwd(LaneType::Sidewalk)], LaneSpecWarnings::default()));
        }
        // Otherwise, there'll always be a bike lane.

        let mut fwd_side = vec![fwd(LaneType::Biking)];
        let mut back_side = if tags.is("oneway", "yes") {
            vec![]
        } else {
            vec![back(LaneType::Biking)]
        };

        if !tags.is("foot", "no") {
            fwd_side.push(fwd(LaneType::Shoulder));
            if !back_side.is_empty() {
                back_side.push(back(LaneType::Shoulder));
            }
        }
        return Ok((
            assemble_ltr(fwd_side, back_side, cfg.driving_side),
            LaneSpecWarnings::default(),
        ));
    }

    // TODO Reversible roads should be handled differently?
    let oneway =
        tags.is_any("oneway", vec!["yes", "reversible"]) || tags.is("junction", "roundabout");

    // How many driving lanes in each direction?
    let num_driving_fwd = if let Some(n) = tags
        .get("lanes:forward")
        .and_then(|num| num.parse::<usize>().ok())
    {
        n
    } else if let Some(n) = tags.get("lanes").and_then(|num| num.parse::<usize>().ok()) {
        if oneway {
            n
        } else if n % 2 == 0 {
            n / 2
        } else {
            // usize division rounds down
            (n / 2) + 1
        }
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
        if oneway {
            base
        } else {
            // lanes=1 but not oneway... what is this supposed to mean?
            base.max(1)
        }
    } else if oneway {
        0
    } else {
        1
    };

    let driving_lane = if tags.is("access", "no")
        && (tags.is("bus", "yes") || tags.is("psv", "yes")) // West Seattle
        || tags
            .get("motor_vehicle:conditional")
            .map(|x| x.starts_with("no"))
            .unwrap_or(false)
            && tags.is("bus", "yes")
    // Example: 3rd Ave in downtown Seattle
    {
        LaneType::Bus
    } else if tags.is("access", "no") || tags.is("highway", "construction") {
        LaneType::Construction
    } else {
        LaneType::Driving
    };

    // These are ordered from the road center, going outwards. Most of the members of fwd_side will
    // have Direction::Forward, but there can be exceptions with two-way cycletracks.
    let mut fwd_side: Vec<LaneSpec> = iter::repeat_with(|| fwd(driving_lane))
        .take(num_driving_fwd)
        .collect();
    let mut back_side: Vec<LaneSpec> = iter::repeat_with(|| back(driving_lane))
        .take(num_driving_back)
        .collect();
    // TODO Fix upstream. https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane
    if tags.is("lanes:both_ways", "1") || tags.is("centre_turn_lane", "yes") {
        fwd_side.insert(0, fwd(LaneType::SharedLeftTurn));
    }

    if driving_lane == LaneType::Construction {
        return Ok((
            assemble_ltr(fwd_side, back_side, cfg.driving_side),
            LaneSpecWarnings::default(),
        ));
    }

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
        let offset = if fwd_side[0].lane_type == LaneType::SharedLeftTurn {
            1
        } else {
            0
        };
        if parts.len() == fwd_side.len() - offset {
            for (idx, part) in parts.into_iter().enumerate() {
                if part == "designated" {
                    fwd_side[idx + offset].lane_type = LaneType::Bus;
                }
            }
        }
    }
    if let Some(spec) = tags
        .get("bus:lanes:backward")
        .or_else(|| tags.get("psv:lanes:backward"))
    {
        let parts: Vec<&str> = spec.split('|').collect();
        if parts.len() == back_side.len() {
            for (idx, part) in parts.into_iter().enumerate() {
                if part == "designated" {
                    back_side[idx].lane_type = LaneType::Bus;
                }
            }
        }
    }

    if tags.is_any("cycleway", vec!["lane", "track"]) {
        fwd_side.push(fwd(LaneType::Biking));
        if !back_side.is_empty() {
            back_side.push(back(LaneType::Biking));
        }
    } else if tags.is_any("cycleway:both", vec!["lane", "track"]) {
        fwd_side.push(fwd(LaneType::Biking));
        back_side.push(back(LaneType::Biking));
    } else {
        // Note here that we look at driving_side frequently, to match up left/right with fwd/back.
        // If we're driving on the right, then right=fwd. Driving on the left, then right=back.
        //
        // TODO Can we express this more simply by referring to a left_side and right_side here?
        if tags.is_any("cycleway:right", vec!["lane", "track"]) {
            if cfg.driving_side == DrivingSide::Right {
                if tags.is("cycleway:right:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    fwd_side.push(back(LaneType::Biking));
                }
                fwd_side.push(fwd(LaneType::Biking));
            } else {
                if tags.is("cycleway:right:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    back_side.push(fwd(LaneType::Biking));
                }
                back_side.push(back(LaneType::Biking));
            }
        }
        if tags.is("cycleway:left", "opposite_lane") || tags.is("cycleway", "opposite_lane") {
            if cfg.driving_side == DrivingSide::Right {
                back_side.push(back(LaneType::Biking));
            } else {
                fwd_side.push(fwd(LaneType::Biking));
            }
        }
        if tags.is_any("cycleway:left", vec!["lane", "opposite_track", "track"]) {
            if cfg.driving_side == DrivingSide::Right {
                if tags.is("cycleway:left:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    back_side.push(fwd(LaneType::Biking));
                    back_side.push(back(LaneType::Biking));
                } else if oneway {
                    fwd_side.insert(0, fwd(LaneType::Biking));
                } else {
                    back_side.push(back(LaneType::Biking));
                }
            } else {
                // TODO This should mimic the logic for right-handed driving, but I need test cases
                // first to do this sanely
                if tags.is("cycleway:left:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    fwd_side.push(back(LaneType::Biking));
                }
                fwd_side.push(fwd(LaneType::Biking));
            }
        }
    }

    // My brain hurts. How does the above combinatorial explosion play with
    // https://wiki.openstreetmap.org/wiki/Proposed_features/cycleway:separation? Let's take the
    // "post-processing" approach.
    // TODO Not attempting left-handed driving yet.
    // TODO A two-way cycletrack on one side of a one-way road will almost definitely break this.
    if let Some(buffer) = tags
        .get("cycleway:right:separation:left")
        .and_then(osm_separation_type)
    {
        // TODO These shouldn't fail, but snapping is imperfect... like around
        // https://www.openstreetmap.org/way/486283205
        if let Some(idx) = fwd_side
            .iter()
            .position(|x| x.lane_type == LaneType::Biking)
        {
            fwd_side.insert(idx, fwd(LaneType::Buffer(buffer)));
        }
    }
    if let Some(buffer) = tags
        .get("cycleway:left:separation:left")
        .and_then(osm_separation_type)
    {
        if let Some(idx) = back_side
            .iter()
            .position(|x| x.lane_type == LaneType::Biking)
        {
            back_side.insert(idx, back(LaneType::Buffer(buffer)));
        }
    }
    if let Some(buffer) = tags
        .get("cycleway:left:separation:right")
        .and_then(osm_separation_type)
    {
        // This is assuming a one-way road. That's why we're not looking at back_side.
        if let Some(idx) = fwd_side
            .iter()
            .position(|x| x.lane_type == LaneType::Biking)
        {
            fwd_side.insert(idx + 1, fwd(LaneType::Buffer(buffer)));
        }
    }

    if driving_lane == LaneType::Driving {
        let has_parking = vec!["parallel", "diagonal", "perpendicular"];
        let parking_lane_fwd = tags.is_any("parking:lane:right", has_parking.clone())
            || tags.is_any("parking:lane:both", has_parking.clone());
        let parking_lane_back = tags.is_any("parking:lane:left", has_parking.clone())
            || tags.is_any("parking:lane:both", has_parking);
        if parking_lane_fwd {
            fwd_side.push(fwd(LaneType::Parking));
        }
        if parking_lane_back {
            back_side.push(back(LaneType::Parking));
        }
    }

    if tags.is("sidewalk", "both") {
        fwd_side.push(fwd(LaneType::Sidewalk));
        back_side.push(back(LaneType::Sidewalk));
    } else if tags.is("sidewalk", "separate") && cfg.inferred_sidewalks {
        // TODO Need to snap separate sidewalks to ways. Until then, just do this.
        fwd_side.push(fwd(LaneType::Sidewalk));
        if !back_side.is_empty() {
            back_side.push(back(LaneType::Sidewalk));
        }
    } else if tags.is("sidewalk", "right") {
        if cfg.driving_side == DrivingSide::Right {
            fwd_side.push(fwd(LaneType::Sidewalk));
        } else {
            back_side.push(back(LaneType::Sidewalk));
        }
    } else if tags.is("sidewalk", "left") {
        if cfg.driving_side == DrivingSide::Right {
            back_side.push(back(LaneType::Sidewalk));
        } else {
            fwd_side.push(fwd(LaneType::Sidewalk));
        }
    }

    let mut need_fwd_shoulder = fwd_side
        .last()
        .map(|spec| spec.lane_type != LaneType::Sidewalk)
        .unwrap_or(true);
    let mut need_back_shoulder = back_side
        .last()
        .map(|spec| spec.lane_type != LaneType::Sidewalk)
        .unwrap_or(true);
    if tags.is_any(HIGHWAY, vec!["motorway", "motorway_link", "construction"])
        || tags.is("foot", "no")
        || tags.is("access", "no")
        || tags.is("motorroad", "yes")
    {
        need_fwd_shoulder = false;
        need_back_shoulder = false;
    }
    // If it's a one-way, fine to not have sidewalks on both sides.
    if tags.is("oneway", "yes") {
        need_back_shoulder = false;
    }

    // For living streets in Krakow, there aren't separate footways. People can walk in the street.
    // For now, model that by putting shoulders.
    if cfg.inferred_sidewalks || tags.is(HIGHWAY, "living_street") {
        if need_fwd_shoulder {
            fwd_side.push(fwd(LaneType::Shoulder));
        }
        if need_back_shoulder {
            back_side.push(back(LaneType::Shoulder));
        }
    }

    Ok((
        (assemble_ltr(fwd_side, back_side, cfg.driving_side)),
        LaneSpecWarnings::default(),
    ))
}

pub fn get_lane_specs_ltr(tags: Tags, cfg: &Config) -> Result<Vec<LaneSpec>, LaneSpecError> {
    let (lane_specs, warnings) = get_lane_specs_ltr_with_warnings(tags, cfg)?;
    if !warnings.0.is_empty() {
        return Err(LaneSpecError(format!(
            "{} warnings found",
            warnings.0.len()
        )));
    }
    Ok(lane_specs)
}

fn assemble_ltr(
    mut fwd_side: Vec<LaneSpec>,
    mut back_side: Vec<LaneSpec>,
    driving_side: DrivingSide,
) -> Vec<LaneSpec> {
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

// See https://wiki.openstreetmap.org/wiki/Proposed_features/cycleway:separation#Typical_values.
// Lots of these mappings are pretty wacky right now. We need more BufferTypes.
fn osm_separation_type(x: &str) -> Option<BufferType> {
    match x {
        "bollard" | "vertical_panel" => Some(BufferType::FlexPosts),
        "kerb" | "separation_kerb" => Some(BufferType::Curb),
        "grass_verge" | "planter" | "tree_row" => Some(BufferType::Planters),
        "guard_rail" | "jersey_barrier" | "railing" => Some(BufferType::JerseyBarrier),
        // TODO Make sure there's a parking lane on that side... also mapped? Any flex posts in
        // between?
        "parking_lane" => None,
        "barred_area" | "dashed_line" | "solid_line" => Some(BufferType::Stripes),
        _ => None,
    }
}
