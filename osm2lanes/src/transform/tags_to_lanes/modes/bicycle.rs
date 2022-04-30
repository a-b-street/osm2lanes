use crate::locale::Locale;
use crate::road::{Designated, Direction};
use crate::tag::Tags;
use crate::transform::tags::CYCLEWAY;
use crate::transform::tags_to_lanes::oneway::Oneway;
use crate::transform::tags_to_lanes::road::LaneType;
use crate::transform::tags_to_lanes::{Infer, LaneBuilder, RoadBuilder, TagsToLanesMsg};
use crate::transform::{RoadWarnings, WaySide};

struct UnknownVariant;

impl Tags {
    fn get_variant<T: AsRef<str>>(&self, k: T) -> Result<Option<Variant>, UnknownVariant> {
        match self.get(k) {
            Some("lane") => Ok(Some(Variant::Lane)),
            Some("track") => Ok(Some(Variant::Track)),
            Some(_) => Err(UnknownVariant),
            None => Ok(None),
        }
    }
    fn cycleway_variant(&self, side: Option<WaySide>) -> Result<Option<Variant>, UnknownVariant> {
        if let Some(side) = side {
            self.get_variant(CYCLEWAY + side.as_str())
        } else {
            self.get_variant(CYCLEWAY)
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(in crate::transform::tags_to_lanes) enum Variant {
    Lane,
    Track,
}

#[derive(Debug, PartialEq)]
pub(in crate::transform::tags_to_lanes) struct Way {
    variant: Variant,
    direction: Direction,
}

#[derive(Debug, PartialEq)]
pub(in crate::transform::tags_to_lanes) enum Location {
    None,
    _No,
    Forward(Way),
    Backward(Way),
    Both { forward: Way, backward: Way },
}

/// Inferred busway scheme for forward lane and backward lane existing
#[derive(Debug)]
pub(in crate::transform::tags_to_lanes) struct Scheme(Location);

impl Scheme {
    #[allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
    pub(in crate::transform::tags_to_lanes) fn from_tags(
        tags: &Tags,
        locale: &Locale,
        road_oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        if let Ok(Some(variant)) = tags.cycleway_variant(None) {
            if tags
                .cycleway_variant(Some(WaySide::Both))
                .ok()
                .flatten()
                .is_some()
                || tags
                    .cycleway_variant(Some(WaySide::Left))
                    .ok()
                    .flatten()
                    .is_some()
                || tags
                    .cycleway_variant(Some(WaySide::Right))
                    .ok()
                    .flatten()
                    .is_some()
            {
                return Err(TagsToLanesMsg::unsupported_str(
                    "cycleway=* with any cycleway:* values",
                ));
            }
            if road_oneway.into() {
                Ok(Self(Location::Forward(Way {
                    variant,
                    direction: Direction::Forward,
                })))
            } else {
                Ok(Self(Location::Both {
                    forward: Way {
                        variant,
                        direction: Direction::Forward,
                    },
                    backward: Way {
                        variant,
                        direction: Direction::Backward,
                    },
                }))
            }
        } else if let Ok(Some(variant)) = tags.cycleway_variant(Some(WaySide::Both)) {
            Ok(Self(Location::Both {
                forward: Way {
                    variant,
                    direction: Direction::Forward,
                },
                backward: Way {
                    variant,
                    direction: Direction::Backward,
                },
            }))
        } else {
            // cycleway=opposite_lane
            if tags.is(CYCLEWAY, "opposite_lane") {
                warnings.push(TagsToLanesMsg::deprecated_tags(
                    tags.subset(&["cycleway", "oneway"]),
                ));
                return Ok(Self(Location::Backward(Way {
                    variant: Variant::Lane,
                    direction: Direction::Backward,
                })));
            }
            // cycleway=opposite oneway=yes oneway:bicycle=no
            if tags.is(CYCLEWAY, "opposite") {
                if !(road_oneway.into() && tags.is("oneway:bicycle", "no")) {
                    return Err(TagsToLanesMsg::unsupported_str(
                        "cycleway=opposite without oneway=yes oneway:bicycle=no",
                    ));
                }
                return Ok(Self(Location::Backward(Way {
                    variant: Variant::Lane,
                    direction: Direction::Backward,
                })));
            }
            // cycleway:FORWARD=*
            if let Ok(Some(variant)) = tags.cycleway_variant(Some(locale.driving_side.into())) {
                if tags.is(CYCLEWAY + locale.driving_side.tag() + "oneway", "no")
                    || tags.is("oneway:bicycle", "no")
                {
                    return Ok(Self(Location::Forward(Way {
                        variant,
                        direction: Direction::Both,
                    })));
                }
                return Ok(Self(Location::Forward(Way {
                    variant,
                    direction: Direction::Forward,
                })));
            }
            // cycleway:FORWARD=opposite_lane
            if tags.is_any(
                CYCLEWAY + locale.driving_side.tag(),
                &["opposite_lane", "opposite_track"],
            ) {
                warnings.push(TagsToLanesMsg::deprecated_tags(
                    tags.subset(&[CYCLEWAY + locale.driving_side.tag()]),
                ));
                return Ok(Self(Location::Forward(Way {
                    variant: Variant::Lane, // TODO distinguish oposite_ values
                    direction: Direction::Backward,
                })));
            }
            // cycleway:BACKWARD=*
            if let Ok(Some(variant)) =
                tags.cycleway_variant(Some(locale.driving_side.opposite().into()))
            {
                return Ok(Self(
                    if tags.is(
                        CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                        "yes",
                    ) {
                        Location::Backward(Way {
                            variant,
                            direction: Direction::Forward,
                        })
                    } else if tags.is(
                        CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                        "-1",
                    ) {
                        Location::Backward(Way {
                            variant,
                            direction: Direction::Backward,
                        })
                    } else if tags.is(
                        CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                        "no",
                    ) || tags.is("oneway:bicycle", "no")
                    {
                        Location::Backward(Way {
                            variant,
                            direction: Direction::Both,
                        })
                    } else if road_oneway.into() {
                        // A oneway road with a cycleway on the wrong side
                        Location::Backward(Way {
                            variant,
                            direction: Direction::Forward,
                        })
                    } else {
                        // A contraflow bicycle lane
                        Location::Backward(Way {
                            variant,
                            direction: Direction::Backward,
                        })
                    },
                ));
            }
            // cycleway:BACKWARD=opposite_lane
            if tags.is_any(
                CYCLEWAY + locale.driving_side.opposite().tag(),
                &["opposite_lane", "opposite_track"],
            ) {
                return Err(TagsToLanesMsg::unsupported_tags(
                    tags.subset(&[CYCLEWAY + locale.driving_side.opposite().tag()]),
                ));
            }
            Ok(Self(Location::None))
        }
    }
}

impl LaneBuilder {
    fn cycle_forward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(Direction::Forward),
            designated: Infer::Direct(Designated::Bicycle),
            ..Default::default()
        }
    }
    fn cycle_backward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(Direction::Backward),
            designated: Infer::Direct(Designated::Bicycle),
            ..Default::default()
        }
    }
    fn cycle_both(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(Direction::Both),
            designated: Infer::Direct(Designated::Bicycle),
            ..Default::default()
        }
    }
}

pub(in crate::transform::tags_to_lanes) fn bicycle(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), TagsToLanesMsg> {
    let scheme = Scheme::from_tags(tags, locale, road.oneway, warnings)?;
    let lane = |way: Way| match way.direction {
        Direction::Forward => LaneBuilder::cycle_forward(locale),
        Direction::Backward => LaneBuilder::cycle_backward(locale),
        Direction::Both => LaneBuilder::cycle_both(locale),
    };
    match scheme.0 {
        Location::None | Location::_No => {},
        Location::Forward(way) => {
            road.push_forward_outside(lane(way));
        },
        Location::Backward(way) => {
            road.push_backward_outside(lane(way));
        },
        Location::Both { forward, backward } => {
            road.push_forward_outside(lane(forward));
            road.push_backward_outside(lane(backward));
        },
    }
    Ok(())
}
