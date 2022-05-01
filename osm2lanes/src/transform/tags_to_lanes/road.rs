use std::collections::VecDeque;
use std::iter;

use super::infer::Infer;
use super::oneway::Oneway;
use super::separator::{
    lane_pair_to_semantic_separator, lane_to_inner_edge_separator, lane_to_outer_edge_separator,
    semantic_separator_to_lane,
};
use super::TagsToLanesMsg;
use crate::locale::{DrivingSide, Locale};
use crate::metric::{Metre, Speed};
use crate::road::{Access as LaneAccess, Designated, Direction, Lane};
use crate::tag::{Access as AccessValue, Highway, TagKey, Tags, HIGHWAY, LIFECYCLE};
use crate::transform::error::{RoadError, RoadWarnings};
use crate::transform::tags_to_lanes::counts::{CentreTurnLaneScheme, Counts};
use crate::transform::tags_to_lanes::modes::BusLaneCount;

#[derive(Debug)]
pub(in crate::transform) struct LaneBuilderError(pub &'static str);

impl std::fmt::Display for LaneBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LaneBuilderError {}

impl std::convert::From<LaneBuilderError> for RoadError {
    fn from(error: LaneBuilderError) -> Self {
        Self::Msg(TagsToLanesMsg::internal(error.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaneType {
    Travel,
    Parking,
    Shoulder,
}

#[derive(Clone, Default, Debug)]
pub struct Width {
    pub min: Infer<Metre>,
    pub target: Infer<Metre>,
    pub max: Infer<Metre>,
}

#[derive(Clone, Default, Debug)]
pub struct Access {
    pub foot: Infer<AccessValue>,
    pub bicycle: Infer<AccessValue>,
    pub taxi: Infer<AccessValue>,
    pub bus: Infer<AccessValue>,
    pub motor: Infer<AccessValue>,
}

impl From<Access> for Option<LaneAccess> {
    fn from(inferred: Access) -> Self {
        if inferred.foot.is_none()
            && inferred.bicycle.is_none()
            && inferred.taxi.is_none()
            && inferred.bus.is_none()
            && inferred.motor.is_none()
        {
            return None;
        }
        Some(LaneAccess {
            foot: inferred.foot.some(),
            bicycle: inferred.bicycle.some(),
            taxi: inferred.taxi.some(),
            bus: inferred.bus.some(),
            motor: inferred.motor.some(),
        })
    }
}

#[derive(Clone, Default, Debug)]
pub struct LaneBuilder {
    pub r#type: Infer<LaneType>,
    // note: direction is always relative to the way
    pub direction: Infer<Direction>,
    pub designated: Infer<Designated>,
    pub width: Width,
    pub max_speed: Infer<Speed>,
    pub access: Access,
}

impl LaneBuilder {
    #[allow(clippy::panic)]
    #[must_use]
    fn build(self) -> Lane {
        let width = self.width.target.some();
        assert!(
            width.unwrap_or(Lane::DEFAULT_WIDTH).val()
                >= self.width.min.some().unwrap_or(Metre::MIN).val()
        );
        assert!(
            width.unwrap_or(Lane::DEFAULT_WIDTH).val()
                <= self.width.max.some().unwrap_or(Metre::MAX).val()
        );
        match self.r#type.some() {
            Some(LaneType::Travel) => {
                let direction = if let Some(Designated::Foot) = self.designated.some() {
                    None
                } else {
                    self.direction.some()
                };
                Lane::Travel {
                    direction,
                    designated: self.designated.some().unwrap(),
                    width,
                    max_speed: self.max_speed.some(),
                    access: self.access.into(),
                }
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

    /// Create a mirrored version of the lane
    #[must_use]
    fn mirror(&self) -> &Self {
        // TODO: this doesn't need to do anything for now
        // check back after v1.0.0 to see if this is still the case
        self
    }
}

pub(in crate::transform) struct RoadBuilder {
    forward_lanes: VecDeque<LaneBuilder>,
    backward_lanes: VecDeque<LaneBuilder>,
    pub highway: Highway,
    pub oneway: Oneway,
}

impl RoadBuilder {
    #[allow(clippy::items_after_statements, clippy::too_many_lines)]
    pub fn from(
        tags: &Tags,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, RoadError> {
        let highway = Highway::from_tags(tags);
        let highway = match highway {
            Err(None) => return Err(TagsToLanesMsg::unsupported_str("way is not highway").into()),
            Err(Some(s)) => return Err(TagsToLanesMsg::unsupported_tag(HIGHWAY, &s).into()),
            Ok(highway) => match highway {
                highway if highway.is_supported() => highway,
                _ => {
                    return Err(TagsToLanesMsg::unimplemented_tags(tags.subset(&LIFECYCLE)).into());
                },
            },
        };

        let oneway = Oneway::from_tags(tags, locale, warnings)?;

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
                warnings.push(TagsToLanesMsg::unsupported_tags(tags.subset(&[MAXSPEED])));
                None
            },
        };

        let width = locale.travel_width(&designated, highway.r#type());
        let width = Width {
            min: Infer::None,
            target: Infer::Default(width),
            max: Infer::None,
        };

        let bus_lane_counts = BusLaneCount::from_tags(tags, locale, oneway, warnings)?;
        let centre_turn_lanes = CentreTurnLaneScheme::from_tags(tags, oneway, locale, warnings);
        let lane_counts = Counts::new(
            tags,
            oneway,
            &highway,
            &centre_turn_lanes,
            &bus_lane_counts,
            locale,
            warnings,
        );
        log::trace!("lane counts: {lane_counts:?}");

        let road = if let Counts::Directional {
            forward,
            backward,
            centre_turn_lane,
        } = lane_counts
        {
            // These are ordered from the road center, going outwards. Most of the members of fwd_side will
            // have Direction::Forward, but there can be exceptions with two-way cycletracks.
            let mut forward_lanes: VecDeque<_> = iter::repeat_with(|| LaneBuilder {
                r#type: Infer::Default(LaneType::Travel),
                direction: Infer::Default(Direction::Forward),
                designated: Infer::Default(designated),
                max_speed: Infer::direct(max_speed),
                width: width.clone(),
                ..Default::default()
            })
            .take(forward.some().unwrap_or(0))
            .collect();
            let backward_lanes = iter::repeat_with(|| LaneBuilder {
                r#type: Infer::Default(LaneType::Travel),
                direction: Infer::Default(Direction::Backward),
                designated: Infer::Default(designated),
                max_speed: Infer::direct(max_speed),
                width: width.clone(),
                ..Default::default()
            })
            .take(backward.some().unwrap_or(0))
            .collect();

            if centre_turn_lane.some().unwrap_or(false) {
                forward_lanes.push_front(LaneBuilder {
                    r#type: Infer::Default(LaneType::Travel),
                    direction: Infer::Default(Direction::Both),
                    designated: Infer::Default(designated),
                    width,
                    ..Default::default()
                });
            }

            RoadBuilder {
                forward_lanes,
                backward_lanes,
                highway,
                oneway,
            }
        } else {
            RoadBuilder {
                forward_lanes: VecDeque::from(vec![LaneBuilder {
                    r#type: Infer::Default(LaneType::Travel),
                    direction: Infer::Default(Direction::Both),
                    designated: Infer::Default(designated),
                    width,
                    ..Default::default()
                }]),
                backward_lanes: VecDeque::new(),
                highway,
                oneway,
            }
        };

        Ok(road)
    }

    /// Number of lanes
    ///
    /// # Panics
    ///
    /// Too many lanes
    pub fn len(&self) -> usize {
        self.forward_len()
            .checked_add(self.backward_len())
            .expect("too many lanes")
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
    pub fn forward_inside_mut(&mut self) -> Option<&mut LaneBuilder> {
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
    pub fn _push_forward_inside(&mut self, lane: LaneBuilder) {
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
    // TODO: a refactor...
    #[allow(
        clippy::needless_collect,
        clippy::unnecessary_wraps,
        clippy::too_many_lines
    )]
    pub fn into_ltr(
        mut self,
        tags: &Tags,
        locale: &Locale,
        include_separators: bool,
        warnings: &mut RoadWarnings,
    ) -> Result<(Vec<Lane>, Highway, Oneway), RoadError> {
        let lanes: Vec<Lane> = if include_separators {
            let forward_edge = self
                .forward_outside()
                .and_then(lane_to_outer_edge_separator);
            let backward_edge = self
                .backward_outside()
                .and_then(lane_to_outer_edge_separator);
            let middle_separator = match [self.forward_inside(), self.backward_inside()] {
                [Some(forward), Some(backward)] => lane_pair_to_semantic_separator(
                    [forward, backward],
                    &self,
                    tags,
                    locale,
                    warnings,
                )
                .and_then(|separator| {
                    semantic_separator_to_lane(
                        [forward, backward],
                        &separator,
                        &self,
                        tags,
                        locale,
                        warnings,
                    )
                }),
                [Some(lane), None] | [None, Some(lane)] => {
                    lane_to_inner_edge_separator(lane.mirror()).map(Lane::mirror)
                },
                [None, None] => return Err(RoadError::Msg(TagsToLanesMsg::internal("no lanes"))),
            };

            self.forward_lanes.make_contiguous();
            let forward_separators: Vec<Option<Lane>> = self
                .forward_lanes
                .as_slices()
                .0
                .windows(2)
                .map(|window| {
                    let lanes: &[LaneBuilder; 2] = window.try_into().unwrap();
                    lane_pair_to_semantic_separator(
                        [&lanes[0], &lanes[1]],
                        &self,
                        tags,
                        locale,
                        warnings,
                    )
                    .and_then(|separator| {
                        semantic_separator_to_lane(
                            [&lanes[0], &lanes[1]],
                            &separator,
                            &self,
                            tags,
                            locale,
                            warnings,
                        )
                    })
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
                    lane_pair_to_semantic_separator(
                        [&lanes[0], &lanes[1]],
                        &self,
                        tags,
                        locale,
                        warnings,
                    )
                    .and_then(|separator| {
                        semantic_separator_to_lane(
                            [&lanes[0], &lanes[1]],
                            &separator,
                            &self,
                            tags,
                            locale,
                            warnings,
                        )
                    })
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
