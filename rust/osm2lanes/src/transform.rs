use std::iter;

use crate::{BufferType, Config, Direction, DrivingSide, LaneSpec, LaneType, Tags};

impl Tags {
    const HIGHWAY: &'static str = "highway";
    fn highway_is(&self, v: &str) -> bool {
        self.is(Self::HIGHWAY, v)
    }
    fn highway_is_any(&self, values: &[&str]) -> bool {
        self.is_any(Self::HIGHWAY, values)
    }
}

impl LaneSpec {
    fn forward(lane_type: LaneType) -> Self {
        Self {
            lane_type,
            direction: Direction::Forward,
        }
    }
    fn backward(lane_type: LaneType) -> Self {
        Self {
            lane_type,
            direction: Direction::Backward,
        }
    }
}

// https://wiki.openstreetmap.org/wiki/Key:access#List_of_possible_values
// enum Access {
//     Yes,
//     No,
//     // Private,
//     // Permissive,
//     // Permit,
//     // Destination,
//     // Delivery,
//     // Customers,
//     Designated,
//     // UseSidepath,
//     Dismount,
//     // Agricultural,
//     // Forestry,
//     // Discouraged,
//     // Unknown,
//     // Other(String),
// }

// https://wiki.openstreetmap.org/wiki/Key:access#Land-based_transportation
// For no good reason, we group non-vehicle, human-powered, and motorized vehicles
// enum NonMotorizedTransport {
//     Foot,
//     Horse,
//     Bicycle,
//     Carriage,
// }

// enum MotorizedVehicleType {
//     Single(SingleTrackedMotorizedVehicle),
//     Double(DoubleTrackedMotorizedVehicle),
//     // Other,
// }

// enum SingleTrackedMotorizedVehicle {
//     Motorcycle,
//     Moped,
//     SpeedPedelec,
//     Mofa,
// }

// enum DoubleTrackedMotorizedVehicle {
//     Motorcar,
//     // Motorhome,
//     // TouristBus,
//     Coach,
//     Goods,
//     Hgv(HeavyGoodsVehicle),
//     // Agricultural,
//     // GolfCart,
//     // Atv,
// }

// enum HeavyGoodsVehicle {
//     Articulated,
//     Bdouble,
//     // Other(String),
// }

// enum MotorizedVehicleUse {
//     Psv(PublicServiceVehicle),
//     Hov,
//     // CarSharing,
//     // Emergency,
//     // Hazmat,
//     // Disabled,
// }

// enum PublicServiceVehicle {
//     Bus,
//     Taxi,
//     Minubs,
//     ShareTaxi,
// }

// Handle non motorized ways
fn non_motorized(tags: &Tags, cfg: &Config) -> Option<Vec<LaneSpec>> {
    if !tags.highway_is_any(&[
        "cycleway",
        "footway",
        "path",
        "pedestrian",
        "steps",
        "track",
    ]) {
        return None;
    }
    // Easy special cases first.
    if tags.highway_is("steps") {
        return Some(vec![LaneSpec::forward(LaneType::Sidewalk)]);
    }

    // Eventually, we should have some kind of special LaneType for shared walking/cycling paths of
    // different kinds. Until then, model by making bike lanes and a shoulder for walking.

    // If it just allows foot traffic, simply make it a sidewalk. For most of the above highway
    // types, assume bikes are allowed, except for footways, where they must be explicitly
    // allowed.
    if tags.is("bicycle", "no")
        || (tags.highway_is("footway")
            && !tags.is_any("bicycle", &["designated", "yes", "dismount"]))
    {
        return Some(vec![LaneSpec::forward(LaneType::Sidewalk)]);
    }
    // Otherwise, there'll always be a bike lane.

    let mut fwd_side = vec![LaneSpec::forward(LaneType::Biking)];
    let mut back_side = if tags.is("oneway", "yes") {
        vec![]
    } else {
        vec![LaneSpec::backward(LaneType::Biking)]
    };

    if !tags.is("foot", "no") {
        fwd_side.push(LaneSpec::forward(LaneType::Shoulder));
        if !back_side.is_empty() {
            back_side.push(LaneSpec::backward(LaneType::Shoulder));
        }
    }
    Some(assemble_ltr(fwd_side, back_side, cfg.driving_side))
}

fn driving_lane_directions(tags: &Tags, _cfg: &Config, oneway: bool) -> (usize, usize) {
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
    (num_driving_fwd, num_driving_back)
}

fn bus(
    tags: &Tags,
    _cfg: &Config,
    oneway: bool,
    forward_side: &mut [LaneSpec],
    backward_side: &mut [LaneSpec],
) {
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
        let offset = if forward_side[0].lane_type == LaneType::SharedLeftTurn {
            1
        } else {
            0
        };
        if parts.len() == forward_side.len() - offset {
            for (idx, part) in parts.into_iter().enumerate() {
                if part == "designated" {
                    forward_side[idx + offset].lane_type = LaneType::Bus;
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
                    backward_side[idx].lane_type = LaneType::Bus;
                }
            }
        }
    }
}

fn bicycle(
    tags: &Tags,
    cfg: &Config,
    oneway: bool,
    forward_side: &mut Vec<LaneSpec>,
    backward_side: &mut Vec<LaneSpec>,
) {
    if tags.is_any("cycleway", &["lane", "track"]) {
        forward_side.push(LaneSpec::forward(LaneType::Biking));
        if !backward_side.is_empty() {
            backward_side.push(LaneSpec::backward(LaneType::Biking));
        }
    } else if tags.is_any("cycleway:both", &["lane", "track"]) {
        forward_side.push(LaneSpec::forward(LaneType::Biking));
        backward_side.push(LaneSpec::backward(LaneType::Biking));
    } else {
        // Note here that we look at driving_side frequently, to match up left/right with fwd/back.
        // If we're driving on the right, then right=fwd. Driving on the left, then right=back.
        //
        // TODO Can we express this more simply by referring to a left_side and right_side here?
        if tags.is_any("cycleway:right", &["lane", "track"]) {
            if cfg.driving_side == DrivingSide::Right {
                if tags.is("cycleway:right:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    forward_side.push(LaneSpec::backward(LaneType::Biking));
                }
                forward_side.push(LaneSpec::forward(LaneType::Biking));
            } else {
                if tags.is("cycleway:right:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    backward_side.push(LaneSpec::forward(LaneType::Biking));
                }
                backward_side.push(LaneSpec::backward(LaneType::Biking));
            }
        }
        if tags.is("cycleway:left", "opposite_lane") || tags.is("cycleway", "opposite_lane") {
            if cfg.driving_side == DrivingSide::Right {
                backward_side.push(LaneSpec::backward(LaneType::Biking));
            } else {
                forward_side.push(LaneSpec::forward(LaneType::Biking));
            }
        }
        if tags.is_any("cycleway:left", &["lane", "opposite_track", "track"]) {
            if cfg.driving_side == DrivingSide::Right {
                if tags.is("cycleway:left:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    backward_side.push(LaneSpec::forward(LaneType::Biking));
                    backward_side.push(LaneSpec::backward(LaneType::Biking));
                } else if oneway {
                    forward_side.insert(0, LaneSpec::forward(LaneType::Biking));
                } else {
                    backward_side.push(LaneSpec::backward(LaneType::Biking));
                }
            } else {
                // TODO This should mimic the logic for right-handed driving, but I need test cases
                // first to do this sanely
                if tags.is("cycleway:left:oneway", "no") || tags.is("oneway:bicycle", "no") {
                    forward_side.push(LaneSpec::backward(LaneType::Biking));
                }
                forward_side.push(LaneSpec::forward(LaneType::Biking));
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
        if let Some(idx) = forward_side
            .iter()
            .position(|x| x.lane_type == LaneType::Biking)
        {
            forward_side.insert(idx, LaneSpec::forward(LaneType::Buffer(buffer)));
        }
    }
    if let Some(buffer) = tags
        .get("cycleway:left:separation:left")
        .and_then(osm_separation_type)
    {
        if let Some(idx) = backward_side
            .iter()
            .position(|x| x.lane_type == LaneType::Biking)
        {
            backward_side.insert(idx, LaneSpec::backward(LaneType::Buffer(buffer)));
        }
    }
    if let Some(buffer) = tags
        .get("cycleway:left:separation:right")
        .and_then(osm_separation_type)
    {
        // This is assuming a one-way road. That's why we're not looking at back_side.
        if let Some(idx) = forward_side
            .iter()
            .position(|x| x.lane_type == LaneType::Biking)
        {
            forward_side.insert(idx + 1, LaneSpec::forward(LaneType::Buffer(buffer)));
        }
    }
}

fn parking(
    tags: &Tags,
    _cfg: &Config,
    _oneway: bool,
    forward_side: &mut Vec<LaneSpec>,
    backward_side: &mut Vec<LaneSpec>,
) {
    let has_parking = vec!["parallel", "diagonal", "perpendicular"];
    let parking_lane_fwd = tags.is_any("parking:lane:right", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    let parking_lane_back = tags.is_any("parking:lane:left", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    if parking_lane_fwd {
        forward_side.push(LaneSpec::forward(LaneType::Parking));
    }
    if parking_lane_back {
        backward_side.push(LaneSpec::backward(LaneType::Parking));
    }
}

fn walking(
    tags: &Tags,
    cfg: &Config,
    _oneway: bool,
    forward_side: &mut Vec<LaneSpec>,
    backward_side: &mut Vec<LaneSpec>,
) {
    if tags.is("sidewalk", "both") {
        forward_side.push(LaneSpec::forward(LaneType::Sidewalk));
        backward_side.push(LaneSpec::backward(LaneType::Sidewalk));
    } else if tags.is("sidewalk", "separate") && cfg.inferred_sidewalks {
        // TODO Need to snap separate sidewalks to ways. Until then, just do this.
        forward_side.push(LaneSpec::forward(LaneType::Sidewalk));
        if !backward_side.is_empty() {
            backward_side.push(LaneSpec::backward(LaneType::Sidewalk));
        }
    } else if tags.is("sidewalk", "right") {
        if cfg.driving_side == DrivingSide::Right {
            forward_side.push(LaneSpec::forward(LaneType::Sidewalk));
        } else {
            backward_side.push(LaneSpec::backward(LaneType::Sidewalk));
        }
    } else if tags.is("sidewalk", "left") {
        if cfg.driving_side == DrivingSide::Right {
            backward_side.push(LaneSpec::backward(LaneType::Sidewalk));
        } else {
            forward_side.push(LaneSpec::forward(LaneType::Sidewalk));
        }
    }

    let mut need_fwd_shoulder = forward_side
        .last()
        .map(|spec| spec.lane_type != LaneType::Sidewalk)
        .unwrap_or(true);
    let mut need_back_shoulder = backward_side
        .last()
        .map(|spec| spec.lane_type != LaneType::Sidewalk)
        .unwrap_or(true);
    if tags.highway_is_any(&["motorway", "motorway_link", "construction"])
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
    if cfg.inferred_sidewalks || tags.highway_is("living_street") {
        if need_fwd_shoulder {
            forward_side.push(LaneSpec::forward(LaneType::Shoulder));
        }
        if need_back_shoulder {
            backward_side.push(LaneSpec::backward(LaneType::Shoulder));
        }
    }
}

/// From an OpenStreetMap way's tags, determine the lanes along the road from left to right.
pub fn get_lane_specs_ltr(tags: &Tags, cfg: &Config) -> Vec<LaneSpec> {
    if let Some(spec) = non_motorized(tags, cfg) {
        return spec;
    }

    let fwd = |lane_type: LaneType| LaneSpec {
        lane_type,
        direction: Direction::Forward,
    };
    let back = |lane_type: LaneType| LaneSpec {
        lane_type,
        direction: Direction::Backward,
    };

    // TODO Reversible roads should be handled differently?
    let oneway = tags.is_any("oneway", &["yes", "reversible"]) || tags.is("junction", "roundabout");

    let (num_driving_fwd, num_driving_back) = driving_lane_directions(tags, cfg, oneway);

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
        return assemble_ltr(fwd_side, back_side, cfg.driving_side);
    }

    bus(tags, cfg, oneway, &mut fwd_side, &mut back_side);

    bicycle(tags, cfg, oneway, &mut fwd_side, &mut back_side);

    if driving_lane == LaneType::Driving {
        parking(tags, cfg, oneway, &mut fwd_side, &mut back_side);
    }

    walking(tags, cfg, oneway, &mut fwd_side, &mut back_side);

    assemble_ltr(fwd_side, back_side, cfg.driving_side)
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
