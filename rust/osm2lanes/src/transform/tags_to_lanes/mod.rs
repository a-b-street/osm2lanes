use std::iter;

use crate::road::{Lane, LaneDesignated, LaneDirection, Marking, MarkingColor, MarkingStyle};
use crate::tag::{Highway, TagKey, Tags};
use crate::{DrivingSide, Locale, Metre};

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

#[derive(Clone, Copy)]
enum Oneway {
    Yes,
    No,
}

impl std::convert::From<bool> for Oneway {
    fn from(oneway: bool) -> Self {
        if oneway {
            Oneway::Yes
        } else {
            Oneway::No
        }
    }
}

impl std::convert::From<Oneway> for bool {
    fn from(oneway: Oneway) -> Self {
        match oneway {
            Oneway::Yes => true,
            Oneway::No => false,
        }
    }
}

// TODO: implement try when this is closed: https://github.com/rust-lang/rust/issues/84277
/// A value with various levels of inference
#[derive(Copy, Clone)]
pub enum Infer<T> {
    None,
    Default(T),
    // Locale(T),
    // Calculated(T),
    Direct(T),
}

impl<T> Infer<T> {
    pub fn some(self) -> Option<T> {
        match self {
            Self::None => None,
            Self::Default(v) => Some(v),
            // Self::Locale(v) => Some(v),
            // Self::Calculated(v) => Some(v),
            Self::Direct(v) => Some(v),
        }
    }
}

impl<T> Default for Infer<T> {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug)]
pub struct LaneBuilderError(&'static str);

impl std::fmt::Display for LaneBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LaneBuilderError {}

impl std::convert::From<LaneBuilderError> for RoadError {
    fn from(error: LaneBuilderError) -> Self {
        Self::Msg(RoadMsg::Internal(error.0))
    }
}

enum LaneType {
    Travel,
    Parking,
    Shoulder,
}

#[derive(Default)]
struct Width {
    min: Infer<Metre>,
    target: Infer<Metre>,
    max: Infer<Metre>,
}

#[derive(Default)]
struct LaneBuilder {
    r#type: Infer<LaneType>,
    direction: Infer<LaneDirection>,
    designated: Infer<LaneDesignated>,
    width: Width,
}

impl LaneBuilder {
    fn build(self) -> Lane {
        let width = self.width.target.some();
        assert!(
            width.unwrap_or(Lane::DEFAULT_WIDTH).val()
                >= self.width.min.some().unwrap_or(Lane::DEFAULT_WIDTH).val()
        );
        assert!(
            width.unwrap_or(Lane::DEFAULT_WIDTH).val()
                <= self.width.max.some().unwrap_or(Lane::DEFAULT_WIDTH).val()
        );
        match self.r#type.some() {
            Some(LaneType::Travel) => Lane::Travel {
                direction: self.direction.some(),
                designated: self.designated.some().unwrap(),
                width,
            },
            Some(LaneType::Parking) => Lane::Parking {
                direction: self.direction.some().unwrap(),
                designated: self.designated.some().unwrap(),
                width,
            },
            Some(LaneType::Shoulder) => Lane::Shoulder { width },
            None => panic!(),
        }
    }
}

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
/// Warnings are produced for situations that maybe result in accurate lanes.
pub fn tags_to_lanes(tags: &Tags, locale: &Locale, config: &TagsToLanesConfig) -> LanesResult {
    let mut warnings = RoadWarnings::default();

    let highway = unsupported(tags, locale, &mut warnings)?;

    // Early return for non-motorized ways (pedestrian paths, cycle paths, etc.)
    if let Some(spec) = non_motorized(tags, locale, highway)? {
        return Ok(spec);
    }

    let oneway = Oneway::from(tags.is("oneway", "yes") || tags.is("junction", "roundabout"));

    let (mut forward_side, mut backward_side) = initial_forward_backward(tags, locale, oneway);

    bus(
        tags,
        locale,
        oneway,
        &mut forward_side,
        &mut backward_side,
        &mut warnings,
    )?;

    bicycle(
        tags,
        locale,
        oneway,
        &mut forward_side,
        &mut backward_side,
        &mut warnings,
    )?;

    parking(tags, locale, oneway, &mut forward_side, &mut backward_side)?;

    foot_and_shoulder(
        tags,
        locale,
        oneway,
        &mut forward_side,
        &mut backward_side,
        &mut warnings,
    )?;

    // Temporary intermediate conversion
    let (forward_side, backward_side) = (
        forward_side
            .into_iter()
            .map(|lane| lane.build())
            .collect::<Vec<_>>(),
        backward_side
            .into_iter()
            .map(|lane| lane.build())
            .collect::<Vec<_>>(),
    );

    let lanes = assemble_ltr(forward_side, backward_side, locale.driving_side)?;

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

fn initial_forward_backward(
    tags: &Tags,
    locale: &Locale,
    oneway: Oneway,
) -> (Vec<LaneBuilder>, Vec<LaneBuilder>) {
    let (num_driving_fwd, num_driving_back) = driving_lane_directions(tags, locale, oneway);

    let designated = if tags.is("access", "no")
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
    let mut fwd_side: Vec<LaneBuilder> = iter::repeat_with(|| LaneBuilder {
        r#type: Infer::Default(LaneType::Travel),
        direction: Infer::Default(LaneDirection::Forward),
        designated: Infer::Default(designated),
        ..Default::default()
    })
    .take(num_driving_fwd)
    .collect();
    let back_side: Vec<LaneBuilder> = iter::repeat_with(|| LaneBuilder {
        r#type: Infer::Default(LaneType::Travel),
        direction: Infer::Default(LaneDirection::Backward),
        designated: Infer::Default(designated),
        ..Default::default()
    })
    .take(num_driving_back)
    .collect();
    // TODO Fix upstream. https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane
    if tags.is("lanes:both_ways", "1") || tags.is("centre_turn_lane", "yes") {
        fwd_side.insert(
            0,
            LaneBuilder {
                r#type: Infer::Default(LaneType::Travel),
                direction: Infer::Default(LaneDirection::Both),
                designated: Infer::Default(designated),
                ..Default::default()
            },
        );
    }

    (fwd_side, back_side)
}

fn driving_lane_directions(tags: &Tags, _locale: &Locale, oneway: Oneway) -> (usize, usize) {
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
        let half = if oneway.into() {
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
        let half = if oneway.into() {
            base
        } else {
            // lanes=1 but not oneway... what is this supposed to mean?
            base.max(1)
        };
        half - both_ways
    } else if tags.is("lanes:bus", "2") {
        if oneway.into() {
            1
        } else {
            2
        }
    } else if oneway.into() {
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

pub fn unsupported(
    tags: &Tags,
    _locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Result<Highway, RoadError> {
    let highway = Highway::from_tags(tags);
    let highway = match highway {
        Err(None) => return Err(RoadMsg::unsupported_str("way is not highway").into()),
        Err(Some(s)) => return Err(RoadMsg::unsupported_tag(HIGHWAY, &s).into()),
        Ok(highway) => match highway {
            highway if highway.is_supported() => highway,
            highway => return Err(RoadMsg::unimplemented_tag(HIGHWAY, &highway.to_string()).into()),
        },
    };

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

    Ok(highway)
}
