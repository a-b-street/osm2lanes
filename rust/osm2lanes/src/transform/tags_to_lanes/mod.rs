use std::collections::VecDeque;
use std::iter;
use std::str::FromStr;

use crate::locale::{DrivingSide, Locale};
use crate::metric::{Metre, Speed};
use crate::road::{Color, Designated, Direction, Lane, Marking, Road, Style};
use crate::tag::{Highway, TagKey, Tags, LIFECYCLE};

mod bicycle;
use bicycle::bicycle;

mod bus;
use bus::bus;

mod foot_shoulder;
use foot_shoulder::foot_and_shoulder;

mod parking;
use parking::parking;

mod separator;
use separator::{lane_to_edge_separator, lanes_to_separator};

mod non_motorized;

use non_motorized::non_motorized;

use crate::transform::tags_to_lanes::bus::{Busway};
use super::{
    ModeResult, RoadError, RoadFromTags, RoadMsg, RoadWarnings, WaySide, CYCLEWAY, HIGHWAY,
    SHOULDER, SIDEWALK,
};

#[non_exhaustive]
pub struct Config {
    pub error_on_warnings: bool,
    pub include_separators: bool,
}

impl Config {
    #[must_use]
    pub fn new(error_on_warnings: bool, include_separators: bool) -> Self {
        Self {
            error_on_warnings,
            include_separators,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            error_on_warnings: false,
            include_separators: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Oneway {
    // TODO support oneway=-1
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
#[derive(Copy, Clone, Debug)]
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
            Self::Default(v) | Self::Direct(v) => Some(v),
        }
    }
    fn direct(some: Option<T>) -> Self {
        match some {
            None => Self::None,
            Some(v) => Self::Direct(v),
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

#[derive(Debug, Clone, Copy)]
enum LaneType {
    Travel,
    Parking,
    Shoulder,
}

#[derive(Clone, Default, Debug)]
struct Width {
    min: Infer<Metre>,
    target: Infer<Metre>,
    max: Infer<Metre>,
}

#[derive(Clone, Default, Debug)]
pub struct LaneBuilder {
    r#type: Infer<LaneType>,
    // note: direction is always relative to the way
    direction: Infer<Direction>,
    designated: Infer<Designated>,
    width: Width,
    max_speed: Infer<Speed>,
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
                max_speed: self.max_speed.some(),
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

struct RoadBuilder {
    forward_lanes: VecDeque<LaneBuilder>,
    backward_lanes: VecDeque<LaneBuilder>,
    pub highway: Highway,
    pub oneway: Oneway,
}

impl RoadBuilder {
    #[allow(clippy::items_after_statements)]
    pub fn from(
        tags: &Tags,
        oneway: Oneway,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, RoadError> {
        let (num_driving_fwd, num_driving_both, num_driving_back) =
            driving_lane_directions(tags, locale, oneway, warnings);

        let designated = if tags.is("access", "no")
            && (tags.is("bus", "yes") || tags.is("psv", "yes")) // West Seattle
            || tags
                .get("motor_vehicle:conditional")
                .map_or(false, |x| x.starts_with("no"))
                && tags.is("bus", "yes")
        // Example: 3rd Ave in downtown Seattle
        {
            Designated::Bus
        } else {
            Designated::Motor
        };

        const MAXSPEED: TagKey = TagKey::from("maxspeed");
        let max_speed = match tags.get(MAXSPEED).map(str::parse::<Speed>).transpose() {
            Ok(max_speed) => max_speed,
            Err(_e) => {
                warnings.push(RoadMsg::Unsupported {
                    description: None,
                    tags: Some(tags.subset(&[MAXSPEED])),
                });
                None
            }
        };

        // These are ordered from the road center, going outwards. Most of the members of fwd_side will
        // have Direction::Forward, but there can be exceptions with two-way cycletracks.
        let mut forward_lanes: VecDeque<_> = iter::repeat_with(|| LaneBuilder {
            r#type: Infer::Default(LaneType::Travel),
            direction: Infer::Default(Direction::Forward),
            designated: Infer::Default(designated),
            max_speed: Infer::direct(max_speed),
            ..Default::default()
        })
        .take(num_driving_fwd)
        .collect();
        let backward_lanes = iter::repeat_with(|| LaneBuilder {
            r#type: Infer::Default(LaneType::Travel),
            direction: Infer::Default(Direction::Backward),
            designated: Infer::Default(designated),
            max_speed: Infer::direct(max_speed),
            ..Default::default()
        })
        .take(num_driving_back)
        .collect();
        // TODO Fix upstream. https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane
        for _ in 0..num_driving_both {
            forward_lanes.push_front(LaneBuilder {
                r#type: Infer::Default(LaneType::Travel),
                direction: Infer::Default(Direction::Both),
                designated: Infer::Default(designated),
                ..Default::default()
            });
        }

        let highway = Highway::from_tags(tags);
        let highway = match highway {
            Err(None) => return Err(RoadMsg::unsupported_str("way is not highway").into()),
            Err(Some(s)) => return Err(RoadMsg::unsupported_tag(HIGHWAY, &s).into()),
            Ok(highway) => match highway {
                highway if highway.is_supported() => highway,
                _ => {
                    return Err(RoadMsg::Unimplemented {
                        description: None,
                        tags: Some(tags.subset(&LIFECYCLE)),
                    }
                    .into());
                }
            },
        };

        Ok(RoadBuilder {
            forward_lanes,
            backward_lanes,
            highway,
            oneway,
        })
    }
    /// Number of lanes
    pub fn len(&self) -> usize {
        self.forward_len() + self.backward_len()
    }
    /// Number of forward lanes
    pub fn forward_len(&self) -> usize {
        self.forward_lanes.len()
    }
    /// Number of backward lanes
    pub fn backward_len(&self) -> usize {
        self.backward_lanes.len()
    }
    /// Get inner-most forward lane
    pub fn forward_inside(&self) -> Option<&LaneBuilder> {
        self.forward_lanes.front()
    }
    /// Get outer-most forward lane
    pub fn forward_outside(&self) -> Option<&LaneBuilder> {
        self.forward_lanes.back()
    }
    /// Get inner-most backward lane
    pub fn backward_inside(&self) -> Option<&LaneBuilder> {
        self.backward_lanes.front()
    }
    /// Get outer-most backward lane
    pub fn backward_outside(&self) -> Option<&LaneBuilder> {
        self.backward_lanes.back()
    }
    /// Get inner-most forward lane
    pub fn _forward_inside_mut(&mut self) -> Option<&mut LaneBuilder> {
        self.forward_lanes.front_mut()
    }
    /// Get outer-most forward lane
    pub fn forward_outside_mut(&mut self) -> Option<&mut LaneBuilder> {
        self.forward_lanes.back_mut()
    }
    /// Get inner-most backward lane
    pub fn _backward_inside_mut(&mut self) -> Option<&mut LaneBuilder> {
        self.backward_lanes.front_mut()
    }
    /// Get outer-most backward lane
    pub fn backward_outside_mut(&mut self) -> Option<&mut LaneBuilder> {
        self.backward_lanes.back_mut()
    }
    /// Push new inner-most forward lane
    pub fn push_forward_inside(&mut self, lane: LaneBuilder) {
        self.forward_lanes.push_front(lane);
    }
    /// Push new outer-most forward lane
    pub fn push_forward_outside(&mut self, lane: LaneBuilder) {
        self.forward_lanes.push_back(lane);
    }
    /// Push new inner-most backward lane
    pub fn _push_backward_inside(&mut self, lane: LaneBuilder) {
        self.backward_lanes.push_front(lane);
    }
    /// Push new outer-most backward lane
    pub fn push_backward_outside(&mut self, lane: LaneBuilder) {
        self.backward_lanes.push_back(lane);
    }
    /// Get lanes left to right
    pub fn lanes_ltr<'a>(&'a self, locale: &Locale) -> Box<dyn Iterator<Item = &LaneBuilder> + 'a> {
        match locale.driving_side {
            DrivingSide::Left => Box::new(
                self.forward_lanes
                    .iter()
                    .rev()
                    .chain(self.backward_lanes.iter()),
            ),
            DrivingSide::Right => Box::new(
                self.backward_lanes
                    .iter()
                    .rev()
                    .chain(self.forward_lanes.iter()),
            ),
        }
    }
    /// Get lanes left to right
    pub fn lanes_ltr_mut<'a>(
        &'a mut self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &mut LaneBuilder> + 'a> {
        match locale.driving_side {
            DrivingSide::Left => Box::new(
                self.forward_lanes
                    .iter_mut()
                    .rev()
                    .chain(self.backward_lanes.iter_mut()),
            ),
            DrivingSide::Right => Box::new(
                self.backward_lanes
                    .iter_mut()
                    .rev()
                    .chain(self.forward_lanes.iter_mut()),
            ),
        }
    }
    /// Get forward lanes left to right
    pub fn _forward_ltr<'a>(
        &'a self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &LaneBuilder> + 'a> {
        match locale.driving_side {
            DrivingSide::Left => Box::new(self.forward_lanes.iter().rev()),
            DrivingSide::Right => Box::new(self.forward_lanes.iter()),
        }
    }
    /// Get forward lanes left to right
    pub fn forward_ltr_mut<'a>(
        &'a mut self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &mut LaneBuilder> + 'a> {
        match locale.driving_side {
            DrivingSide::Left => Box::new(self.forward_lanes.iter_mut().rev()),
            DrivingSide::Right => Box::new(self.forward_lanes.iter_mut()),
        }
    }
    /// Get backward lanes left to right
    pub fn _backward_ltr<'a>(
        &'a self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &LaneBuilder> + 'a> {
        match locale.driving_side {
            DrivingSide::Left => Box::new(self.backward_lanes.iter().rev()),
            DrivingSide::Right => Box::new(self.backward_lanes.iter()),
        }
    }
    /// Get backward lanes left to right
    pub fn backward_ltr_mut<'a>(
        &'a mut self,
        locale: &Locale,
    ) -> Box<dyn Iterator<Item = &mut LaneBuilder> + 'a> {
        match locale.driving_side {
            DrivingSide::Left => Box::new(self.backward_lanes.iter_mut().rev()),
            DrivingSide::Right => Box::new(self.backward_lanes.iter_mut()),
        }
    }

    /// Consume Road Builder to return Lanes left to right
    #[allow(clippy::needless_collect, clippy::unnecessary_wraps)]
    pub fn into_ltr(
        mut self,
        tags: &Tags,
        locale: &Locale,
        include_separators: bool,
        warnings: &mut RoadWarnings,
    ) -> Result<(Vec<Lane>, Highway, Oneway), RoadError> {
        let lanes: Vec<Lane> = if include_separators {
            let forward_edge = lane_to_edge_separator(self.forward_outside().unwrap());
            let backward_edge = lane_to_edge_separator(self.backward_outside().unwrap());
            let middle_separator = lanes_to_separator(
                [
                    self.forward_inside().unwrap(),
                    self.backward_inside().unwrap(),
                ],
                &self,
                tags,
                locale,
                warnings,
            );

            self.forward_lanes.make_contiguous();
            let forward_separators: Vec<Option<Lane>> = self
                .forward_lanes
                .as_slices()
                .0
                .windows(2)
                .map(|window| {
                    let lanes: &[LaneBuilder; 2] = window.try_into().unwrap();
                    lanes_to_separator([&lanes[0], &lanes[1]], &self, tags, locale, warnings)
                })
                .collect();

            self.backward_lanes.make_contiguous();
            let backward_separators: Vec<Option<Lane>> = self
                .backward_lanes
                .as_slices()
                .0
                .windows(2)
                .map(|window| {
                    let lanes: &[LaneBuilder; 2] = window.try_into().unwrap();
                    lanes_to_separator([&lanes[0], &lanes[1]], &self, tags, locale, warnings)
                })
                .collect();

            let forward_lanes_with_separators: Vec<Option<Lane>> = self
                .forward_lanes
                .into_iter()
                .map(LaneBuilder::build)
                .map(Some)
                .zip(
                    forward_separators
                        .into_iter()
                        .chain(iter::once(forward_edge)),
                )
                .flat_map(|(a, b)| [a, b])
                .collect();
            let backward_lanes_with_separators: Vec<Option<Lane>> = self
                .backward_lanes
                .into_iter()
                .map(LaneBuilder::build)
                .map(Some)
                .zip(
                    backward_separators
                        .into_iter()
                        .chain(iter::once(backward_edge)),
                )
                .flat_map(|(a, b)| [a, b])
                .collect();

            match locale.driving_side {
                DrivingSide::Left => forward_lanes_with_separators
                    .into_iter()
                    .rev()
                    .chain(iter::once(middle_separator))
                    .chain(backward_lanes_with_separators)
                    .flatten()
                    .collect(),
                DrivingSide::Right => backward_lanes_with_separators
                    .into_iter()
                    .rev()
                    .chain(iter::once(middle_separator))
                    .chain(forward_lanes_with_separators)
                    .flatten()
                    .collect(),
            }
        } else {
            match locale.driving_side {
                DrivingSide::Left => self
                    .forward_lanes
                    .into_iter()
                    .rev()
                    .chain(self.backward_lanes.into_iter())
                    .map(LaneBuilder::build)
                    .collect(),
                DrivingSide::Right => self
                    .backward_lanes
                    .into_iter()
                    .rev()
                    .chain(self.forward_lanes.into_iter())
                    .map(LaneBuilder::build)
                    .collect(),
            }
        };
        Ok((lanes, self.highway, self.oneway))
    }
}

/// From an OpenStreetMap way's tags,
/// determine the lanes along the road from left to right.
///
/// # Errors
///
/// Warnings or errors are produced for situations that may make the lanes inaccurate, such as:
///
/// - Unimplemented or sunuspported tags
/// - Ambiguous tags
/// - Unknown internal errors
///
/// If the issue may be recoverable, a warning is preferred.
/// A config option allows all warnings to be treated as errors.
///
pub fn tags_to_lanes(
    tags: &Tags,
    locale: &Locale,
    config: &Config,
) -> Result<RoadFromTags, RoadError> {
    let mut warnings = RoadWarnings::default();

    // Early return if we find unimplemented tags.
    unsupported(tags, locale, &mut warnings)?;

    // Parse lane count schemas first.
    let oneway =
        Oneway::from(tags.is_any("oneway", &["yes", "-1"]) || tags.is("junction", "roundabout"));
    let busway = Busway::from(tags, locale, &oneway, &mut warnings);
    // let bus_lanes = BusLanes::from(tags, locale, &oneway, &mut warnings);
    // let lanes_designated = LanesDesignated::from(tags, locale, &oneway, &mut warnings);
    //
    // let lanes = Lanes::from(road, tags);

    // TODO: then check for incompatabilities between schemes, and fill in assumptions/guesses

    // TODO: then add them into the road builder.

    let mut road: RoadBuilder = RoadBuilder::from(tags, oneway, locale, &mut warnings)?;

    // Early return for non-motorized ways (pedestrian paths, cycle paths, etc.)
    if let Some(spec) = non_motorized(tags, locale, &road)? {
        return Ok(spec);
    }

    road.set_busway_scheme(&busway, &locale, &mut warnings)?;

    bus(tags, locale, &mut road, &mut warnings)?;

    bicycle(tags, locale, &mut road, &mut warnings)?;

    parking(tags, locale, &mut road)?;

    foot_and_shoulder(tags, locale, &mut road, &mut warnings)?;

    let (lanes, highway, _oneway) =
        road.into_ltr(tags, locale, config.include_separators, &mut warnings)?;

    let road_from_tags = RoadFromTags {
        road: Road { lanes, highway },
        warnings,
    };

    if config.error_on_warnings && !road_from_tags.warnings.is_empty() {
        return Err(road_from_tags.warnings.into());
    }

    Ok(road_from_tags)
}

impl Tags {
    /// Gets the value for the given key and parses it into T.
    fn get_and_parse<T: FromStr>(&self, key: &str) -> Option<T> {
        self.get(key).and_then(|val| val.parse::<T>().ok())
    }

    /// Gets the value for the given key and parses it into T. A RoadMsg::Unsupported is added if
    /// parsing fails.
    fn get_parsed<T: FromStr>(&self, key: &str, warnings: &mut RoadWarnings) -> Option<T> {
        self.get(key).and_then(|val| match val.parse::<T>() {
            Ok(n) => Some(n),
            Err(_) => {
                warnings.push(RoadMsg::unsupported_tag(key.to_owned(), val));
                None
            }
        })
    }
}

/// Calculates the number of vehicle travel lanes in each direction (forward, both ways, backward)
/// according to the `lanes` schema (which excludes parking lanes, bike lanes, etc.).
/// See https://wiki.openstreetmap.org/wiki/Key:lanes.
///
/// Validates `lanes[:{forward,both_ways,backward}]=*` to determine the precise answer.
/// Uses `lanes:{bus,psv}[:{forward,backward}]=*`, `busway[:{left,both,right}]=*` and
/// `centre_turn_lane=yes` to make assumptions in the absence of precise tagging.
fn driving_lane_directions(
    tags: &Tags,
    locale: &Locale,
    oneway: Oneway,
    warnings: &mut RoadWarnings,
) -> (usize, usize, usize) {
    // The tags for this schema (which we will validate).
    let tagged_lanes = tags.get_parsed::<usize>("lanes", warnings);
    let tagged_forward = tags.get_parsed::<usize>("lanes:forward", warnings);
    let tagged_backward = tags.get_parsed::<usize>("lanes:backward", warnings);
    let tagged_both_ways = tags.get_parsed::<usize>("lanes:both_ways", warnings);

    // TODO? if any lanes:*= tags are present, warn about missing lanes=* tag.

    // lanes:{bus,psv} for guessing
    let tagged_bus_lanes = tags
        .get_and_parse::<usize>("lanes:bus")
        .or(tags.get_and_parse("lanes:psv"));
    let tagged_bus_forward = tags
        .get_and_parse::<usize>("lanes:bus:forward")
        .or(tags.get_and_parse("lanes:psv:forward"));
    let tagged_bus_backward = tags
        .get_and_parse::<usize>("lanes:bus:backward")
        .or(tags.get_and_parse("lanes:psv:backward"));

    // Centre turn lanes
    let tagged_center_turn_lanes = if tags.is("centre_turn_lane", "yes") {
        Some(1)
    } else {
        None
    };
    // Always assume no center turn lane unless tagged, so we already know:
    let num_both_ways = tagged_both_ways.unwrap_or(tagged_center_turn_lanes.unwrap_or(0));

    if oneway.into() {
        // Ignore lanes:{forward,both_ways,backward}=* and centre_turn_lanes=*
        if tagged_both_ways.is_some()
            || tagged_backward.is_some()
            || tagged_center_turn_lanes.is_some()
        {
            warnings.push(RoadMsg::Ambiguous {
                description: None,
                tags: Some(tags.subset(&[
                    "oneway",
                    "lanes:both_ways",
                    "lanes:backward",
                    "centre_turn_lanes",
                ])),
            });
        }
        // The wiki suggests that contraflow bus lanes can be specified on oneway roads
        // let contraflow_bus = if tags.is("busway", "opposite_lane")
        //     || tags.is("busway:left", "opposite_lane")
        //     || tags.is("busway:right", "opposite_lane")
        //     || tags.is("busway:both", "lane")
        // {
        //     1
        // } else {
        //     0
        // };
        if let Some(l) = tagged_lanes {
            if tagged_forward.map_or(false, |f| f != l) {
                warnings.push(RoadMsg::Ambiguous {
                    description: None,
                    tags: Some(tags.subset(&["oneway", "lanes", "lanes:forward"])),
                });
            }
            (l, 0, 0)
        } else {
            let assumed_forward = tagged_forward.unwrap_or(1);
            let mut assumed_extra_bus = tagged_bus_lanes.unwrap_or(0);
            if tags.is("busway", "lane") || tags.is("busway:both", "lane") {
                assumed_extra_bus += 1;
            }
            (assumed_forward + assumed_extra_bus, 0, 0)
        }
    } else {
        //  busway
        const BUSWAY: TagKey = TagKey::from("busway");
        let mut busway_forward_lanes = 0;
        let mut busway_backward_lanes = 0;
        if tags.is("busway", "lane") || tags.is("busway:both", "lane") {
            busway_forward_lanes += 1;
            busway_backward_lanes += 1;
        }
        if tags.is_any(
            BUSWAY + locale.driving_side.tag(),
            &["lane", "opposite_lane"],
        ) {
            busway_forward_lanes += 1;
        }
        if tags.is_any(
            BUSWAY + locale.driving_side.opposite().tag(),
            &["lane", "opposite_lane"],
        ) {
            busway_backward_lanes += 1;
        }

        match (tagged_lanes, tagged_forward, tagged_backward) {
            (_, Some(f), Some(b)) => {
                if let Some(num_lanes) = tagged_lanes {
                    if num_lanes != f + b + num_both_ways {
                        warnings.push(RoadMsg::Ambiguous {
                            description: None,
                            tags: Some(tags.subset(&[
                                "lanes",
                                "lanes:forward",
                                "lanes:both_ways",
                                "lanes:backward",
                            ])),
                        });
                    }
                }
                (f, num_both_ways, b)
            }
            (Some(l), Some(f), None) => (f, num_both_ways, l - f - num_both_ways),
            (Some(l), None, Some(b)) => (l - b - num_both_ways, num_both_ways, b),
            (Some(1), None, None) => (0, 1, 0),
            (Some(l), None, None) => {
                if l % 2 == 0 && tagged_center_turn_lanes.is_some() {
                    // Only tagged with lanes and deprecated center_turn_lane tag.
                    // Assume the center_turn_lane is in addition to evenly divided lanes
                    (l / 2, tagged_center_turn_lanes.unwrap(), l / 2)
                } else {
                    // Count up bus and both way lanes, then divide the remaining evenly.
                    // TODO Ignoring bus:lanes and lanes:bus for now
                    let remaining_lanes =
                        l - num_both_ways - busway_forward_lanes - busway_backward_lanes;
                    if remaining_lanes % 2 != 0 {
                        warnings.push(RoadMsg::Ambiguous {
                            description: Some(String::from("Total lane count cannot be evenly divided between the forward and backward")),
                            tags: Some(tags.subset(&[
                                "lanes",
                                "lanes:both_ways",
                            ])),
                        });
                    }
                    let half = (remaining_lanes + 1) / 2; // usize division rounded up.
                    (
                        half + busway_forward_lanes,
                        num_both_ways,
                        remaining_lanes - half + busway_backward_lanes,
                    )
                }
            }
            (None, _, _) => {
                // Tagging only lanes:forward or lanes:backward is silly, but lets use them.
                let f = tagged_forward.unwrap_or(1);
                let b = tagged_backward.unwrap_or(1);
                // Without the "lanes" tag, assume bus lanes add onto the assumed single lane
                let bus = tagged_bus_lanes.unwrap_or(0);
                let bus_forward =
                    busway_forward_lanes + tagged_bus_forward.unwrap_or((bus + 1) / 2);
                let bus_backward = busway_backward_lanes + tagged_bus_backward.unwrap_or((bus) / 2);
                (f + bus_forward, num_both_ways, b + bus_backward)
            }
        }
    }
}

pub fn unsupported(
    tags: &Tags,
    _locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
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
