use std::iter;

use serde::{Deserialize, Serialize};

use crate::tags::{TagKey, Tags, TagsRead, TagsWrite};
use crate::{DrivingSide, Lane, LaneDesignated, LaneDirection, Locale, Road};

const HIGHWAY: TagKey = TagKey::from("highway");
const CYCLEWAY: TagKey = TagKey::from("cycleway");

#[derive(Clone, Debug, PartialEq)]
enum WaySide {
    Both,
    Right,
    Left,
}

impl WaySide {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Right => "right",
            Self::Left => "left",
        }
    }
}

impl ToString for WaySide {
    fn to_string(&self) -> String {
        self.as_str().to_owned()
    }
}

impl std::convert::From<DrivingSide> for WaySide {
    fn from(side: DrivingSide) -> Self {
        match side {
            DrivingSide::Right => Self::Right,
            DrivingSide::Left => Self::Left,
        }
    }
}

impl std::convert::From<DrivingSide> for TagKey {
    fn from(side: DrivingSide) -> Self {
        match side {
            DrivingSide::Right => Self::from("right"),
            DrivingSide::Left => Self::from("left"),
        }
    }
}

impl DrivingSide {
    fn tag(&self) -> TagKey {
        (*self).into()
    }
}

impl Lane {
    fn forward(designated: LaneDesignated) -> Self {
        Self::Travel {
            direction: Some(LaneDirection::Forward),
            designated,
        }
    }
    fn backward(designated: LaneDesignated) -> Self {
        Self::Travel {
            direction: Some(LaneDirection::Backward),
            designated,
        }
    }
    fn both(designated: LaneDesignated) -> Self {
        Self::Travel {
            direction: Some(LaneDirection::Both),
            designated,
        }
    }
    fn foot() -> Self {
        Self::Travel {
            direction: None,
            designated: LaneDesignated::Foot,
        }
    }
    fn parking(direction: LaneDirection) -> Self {
        Self::Parking {
            direction,
            designated: LaneDesignated::Motor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneSpecError(String);

impl ToString for LaneSpecError {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LaneSpecWarnings(Vec<LaneSpecWarning>);

impl LaneSpecWarnings {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl ToString for LaneSpecWarnings {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|warn| format!("Warning: {}", warn.description))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneSpecWarning {
    pub description: String,
    // Tags relevant to triggering the warning
    // TODO: investigate making this a view of keys on a Tags object instead
    pub tags: Tags,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lanes {
    pub lanes: Vec<Lane>,
    pub warnings: LaneSpecWarnings,
}

type LanesResult = Result<Lanes, LaneSpecError>;
type RoadResult = Result<Road, LaneSpecError>;

// Handle non motorized ways
fn non_motorized(tags: &Tags, lc: &Locale) -> Option<LanesResult> {
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
            warnings: LaneSpecWarnings(vec![LaneSpecWarning {
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
            warnings: LaneSpecWarnings::default(),
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
        lanes: assemble_ltr(forward_side, backward_side, lc.driving_side),
        warnings: LaneSpecWarnings::default(),
    }))
}

fn driving_lane_directions(tags: &Tags, _lc: &Locale, oneway: bool) -> (usize, usize) {
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
    } else if oneway {
        0
    } else {
        1
    };
    (num_driving_fwd, num_driving_back)
}

fn bus(
    tags: &Tags,
    _lc: &Locale,
    oneway: bool,
    forward_side: &mut [Lane],
    backward_side: &mut [Lane],
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
}

fn bicycle(
    tags: &Tags,
    lc: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
    warnings: &mut LaneSpecWarnings,
) -> Result<(), LaneSpecError> {
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
            return Err(LaneSpecError(
                "cycleway=* not supported with any cycleway:* values".to_owned(),
            ));
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
            backward_side.push(Lane::forward(LaneDesignated::Bicycle));
        }
    } else if tags.is_cycleway(Some(WaySide::Both)) {
        forward_side.push(Lane::both(LaneDesignated::Bicycle));
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
        if tags.is_cycleway(Some(lc.driving_side.into())) {
            if tags.is(CYCLEWAY + lc.driving_side.tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
            {
                forward_side.push(Lane::both(LaneDesignated::Bicycle));
            } else {
                forward_side.push(Lane::forward(LaneDesignated::Bicycle));
            }
        }
        // cycleway:FORWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + lc.driving_side.tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            warnings.0.push(LaneSpecWarning {
                description: "cycleway:FORWARD=opposite_lane deprecated".to_owned(),
                tags: tags.subset(&[CYCLEWAY]), // TODO make side specific
            });
            forward_side.push(Lane::backward(LaneDesignated::Bicycle));
        }
        // cycleway:BACKWARD=*
        if tags.is_cycleway(Some(lc.driving_side.opposite().into())) {
            if tags.is(CYCLEWAY + lc.driving_side.opposite().tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
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
            CYCLEWAY + lc.driving_side.opposite().tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            return Err(LaneSpecError(
                "cycleway:BACKWARD=opposite_lane unsupported".to_owned(),
            ));
        }
    }
    Ok(())
}

fn parking(
    tags: &Tags,
    _lc: &Locale,
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

fn walking(
    tags: &Tags,
    lc: &Locale,
    _oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) {
    if tags.is("sidewalk", "both") {
        forward_side.push(Lane::foot());
        backward_side.push(Lane::foot());
    } else if tags.is("sidewalk", "separate") && lc.infer_sidewalks {
        // TODO Need to snap separate sidewalks to ways. Until then, just do this.
        forward_side.push(Lane::foot());
        if !backward_side.is_empty() {
            backward_side.push(Lane::foot());
        }
    } else if tags.is("sidewalk", "right") {
        if lc.driving_side == DrivingSide::Right {
            forward_side.push(Lane::foot());
        } else {
            backward_side.push(Lane::foot());
        }
    } else if tags.is("sidewalk", "left") {
        if lc.driving_side == DrivingSide::Right {
            backward_side.push(Lane::foot());
        } else {
            forward_side.push(Lane::foot());
        }
    }

    let mut need_fwd_shoulder = forward_side
        .last()
        .map(|spec| {
            !matches!(
                spec,
                Lane::Travel {
                    designated: LaneDesignated::Foot,
                    ..
                }
            )
        })
        .unwrap_or(true);
    let mut need_back_shoulder = backward_side
        .last()
        .map(|spec| {
            !matches!(
                spec,
                Lane::Travel {
                    designated: LaneDesignated::Foot,
                    ..
                }
            )
        })
        .unwrap_or(true);
    if tags.is_any(HIGHWAY, &["motorway", "motorway_link", "construction"])
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
    if lc.infer_sidewalks || tags.is(HIGHWAY, "living_street") {
        if need_fwd_shoulder {
            forward_side.push(Lane::Shoulder);
        }
        if need_back_shoulder {
            backward_side.push(Lane::Shoulder);
        }
    }
}

/// From an OpenStreetMap way's tags, determine the lanes along the road from left to right.
pub fn get_lane_specs_ltr_with_warnings(tags: &Tags, lc: &Locale) -> LanesResult {
    let mut warnings = LaneSpecWarnings::default();

    if let Some(spec) = non_motorized(tags, lc) {
        return spec;
    }

    // TODO Reversible roads should be handled differently?
    let oneway = tags.is_any("oneway", &["yes", "reversible"]) || tags.is("junction", "roundabout");

    let (num_driving_fwd, num_driving_back) = driving_lane_directions(tags, lc, oneway);

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

    bus(tags, lc, oneway, &mut fwd_side, &mut back_side);

    bicycle(
        tags,
        lc,
        oneway,
        &mut fwd_side,
        &mut back_side,
        &mut warnings,
    )?;

    if driving_lane == LaneDesignated::Motor {
        parking(tags, lc, oneway, &mut fwd_side, &mut back_side);
    }

    walking(tags, lc, oneway, &mut fwd_side, &mut back_side);

    Ok(Lanes {
        lanes: assemble_ltr(fwd_side, back_side, lc.driving_side),
        warnings,
    })
}

pub fn get_lane_specs_ltr(tags: &Tags, lc: &Locale) -> RoadResult {
    let Lanes { lanes, warnings } = get_lane_specs_ltr_with_warnings(tags, lc)?;
    if !warnings.0.is_empty() {
        return Err(LaneSpecError(format!(
            "{} warnings found",
            warnings.0.len()
        )));
    }
    Ok(Road { lanes })
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

pub fn lanes_to_tags(lanes: &[Lane], lc: &Locale) -> Result<Tags, LaneSpecError> {
    let mut tags = Tags::default();
    let mut _oneway = false;
    tags.insert("highway", "yes"); // TODO, what?
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
        tags.insert("lanes", lane_count.to_string());
    }
    // Oneway
    if lanes
        .iter()
        .filter(|lane| {
            matches!(
                lane,
                Lane::Travel {
                    designated: LaneDesignated::Motor,
                    ..
                }
            )
        })
        .all(|lane| {
            matches!(
                lane,
                Lane::Travel {
                    direction: Some(LaneDirection::Forward),
                    ..
                }
            )
        })
    {
        tags.insert("oneway", "yes");
        _oneway = true;
    }
    // Pedestrian
    {
        match (
            matches!(
                lanes.first().unwrap(),
                Lane::Travel {
                    designated: LaneDesignated::Foot,
                    ..
                }
            ),
            matches!(
                lanes.last().unwrap(),
                Lane::Travel {
                    designated: LaneDesignated::Foot,
                    ..
                }
            ),
        ) {
            (false, false) => {}
            (true, false) => assert!(tags.insert("sidewalk", "left").is_none()),
            (false, true) => assert!(tags.insert("sidewalk", "right").is_none()),
            (true, true) => assert!(tags.insert("sidewalk", "both").is_none()),
        }
    }
    // Parking
    match (
        lanes
            .iter()
            .take_while(|lane| {
                !matches!(
                    lane,
                    Lane::Travel {
                        designated: LaneDesignated::Motor,
                        ..
                    }
                )
            })
            .any(|lane| matches!(lane, Lane::Parking { .. })),
        lanes
            .iter()
            .skip_while(|lane| {
                !matches!(
                    lane,
                    Lane::Travel {
                        designated: LaneDesignated::Motor,
                        ..
                    }
                )
            })
            .any(|lane| matches!(lane, Lane::Parking { .. })),
    ) {
        (false, false) => {}
        (true, false) => assert!(tags.insert("parking:lane:left", "parallel").is_none()),
        (false, true) => assert!(tags.insert("parking:lane:right", "parallel").is_none()),
        (true, true) => assert!(tags.insert("parking:lane:both", "parallel").is_none()),
    }
    // Cycleway
    {
        let left_cycle_lane = lanes
            .iter()
            .take_while(|lane| {
                !matches!(
                    lane,
                    Lane::Travel {
                        designated: LaneDesignated::Motor,
                        ..
                    }
                )
            })
            .find(|lane| {
                matches!(
                    lane,
                    Lane::Travel {
                        designated: LaneDesignated::Bicycle,
                        ..
                    }
                )
            });
        let right_cycle_lane = lanes
            .iter()
            .rev()
            .take_while(|lane| {
                !matches!(
                    lane,
                    Lane::Travel {
                        designated: LaneDesignated::Motor,
                        ..
                    }
                )
            })
            .find(|lane| {
                matches!(
                    lane,
                    Lane::Travel {
                        designated: LaneDesignated::Bicycle,
                        ..
                    }
                )
            });
        match (left_cycle_lane.is_some(), right_cycle_lane.is_some()) {
            (false, false) => {}
            (true, false) => assert!(tags.insert("cycleway:left", "lane").is_none()),
            (false, true) => assert!(tags.insert("cycleway:right", "lane").is_none()),
            (true, true) => assert!(tags.insert("cycleway:both", "lane").is_none()),
        }
        // https://wiki.openstreetmap.org/wiki/Key:cycleway:right:oneway
        // TODO, incomplete, pending testing.
        if let Some(Lane::Travel {
            direction: Some(LaneDirection::Both),
            ..
        }) = left_cycle_lane
        {
            tags.insert("cycleway:left:oneway", "no");
        }
        if let Some(Lane::Travel {
            direction: Some(LaneDirection::Both),
            ..
        }) = right_cycle_lane
        {
            tags.insert("cycleway:right:oneway", "no");
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
        tags.insert("lanes:both_ways", "1");
        // TODO: add LHT support
        tags.insert("turn:lanes:both_ways", "left");
    }

    // Check roundtrip!
    let rountrip = get_lane_specs_ltr(&tags, lc)?;
    if lanes != rountrip.lanes {
        return Err(LaneSpecError("lanes to tags cannot roundtrip".to_owned()));
    }

    Ok(tags)
}
