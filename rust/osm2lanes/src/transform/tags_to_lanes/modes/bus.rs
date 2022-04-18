use crate::locale::Locale;
use crate::metric::Metre;
use crate::road::{Designated, Direction};
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::access_by_lane::Access;
use crate::transform::tags_to_lanes::{
    Infer, LaneBuilder, LaneBuilderError, LaneType, RoadBuilder, Width,
};
use crate::transform::{RoadError, RoadMsg, RoadWarnings};

const LANES: TagKey = TagKey::from("lanes");
const ONEWAY: TagKey = TagKey::from("oneway");
const ONEWAY_BUS: TagKey = TagKey::from("oneway:bus");

impl LaneBuilder {
    #[allow(clippy::unnecessary_wraps)]
    fn set_bus(&mut self, _locale: &Locale) -> Result<(), LaneBuilderError> {
        self.designated = Infer::Direct(Designated::Bus);
        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn get_bus(_locale: &Locale, direction: Direction) -> Result<LaneBuilder, LaneBuilderError> {
    Ok(LaneBuilder {
        r#type: Infer::Direct(LaneType::Travel),
        direction: Infer::Direct(direction),
        designated: Infer::Direct(Designated::Bus),
        width: Width {
            min: Infer::Default(Metre::new(3.0)),
            target: Infer::Default(Metre::new(4.0)),
            max: Infer::Default(Metre::new(4.5)),
        },
        max_speed: Infer::None,
    })
}

#[derive(Debug)]
pub(in crate::transform::tags_to_lanes) enum Scheme {
    None,
    // Busway, handled upfront
    LanesBus,
    BusLanes,
}

#[allow(clippy::unnecessary_wraps)]
#[must_use]
pub(in crate::transform::tags_to_lanes) fn check_bus(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<Scheme, RoadError> {
    // https://wiki.openstreetmap.org/wiki/Bus_lanes
    // 3 schemes,
    // busway is handled before lanes are constructed and the others after

    if tags.tree().get("busway").is_some() {
        busway(tags, locale, road, warnings)?;
    }

    let bus_scheme = match (
        tags.tree()
            .get("lanes:bus")
            .or_else(|| tags.tree().get("lanes:psv"))
            .is_some(),
        tags.tree()
            .get("bus:lanes")
            .or_else(|| tags.tree().get("psv:lanes"))
            .is_some(),
    ) {
        (false, false) => Ok(Scheme::None),
        (true, false) => Ok(Scheme::LanesBus),
        (false, true) => Ok(Scheme::BusLanes),
        (true, true) => Err(RoadMsg::Unsupported {
            description: Some("more than one bus lanes scheme used".to_owned()),
            tags: None,
        }
        .into()),
    };
    log::trace!("bus_scheme={bus_scheme:?}");
    bus_scheme
}

fn busway(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    const BUSWAY: TagKey = TagKey::from("busway");
    if tags.is(BUSWAY, "lane") {
        road.push_forward_outside(get_bus(locale, Direction::Forward)?);
        if !tags.is("oneway", "yes") && !tags.is("oneway:bus", "yes") {
            road.push_backward_outside(get_bus(locale, Direction::Backward)?);
        }
    }
    if tags.is(BUSWAY, "opposite_lane") {
        road.push_backward_outside(get_bus(locale, Direction::Backward)?);
    }
    if tags.is(BUSWAY + "both", "lane") {
        road.push_forward_outside(get_bus(locale, Direction::Forward)?);
        road.push_backward_outside(get_bus(locale, Direction::Backward)?);
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            warnings.push(RoadMsg::Ambiguous {
                description: None,
                tags: Some(tags.subset(&[BUSWAY + "both", ONEWAY, ONEWAY_BUS])),
            });
        }
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "lane") {
        road.push_forward_outside(get_bus(locale, Direction::Forward)?);
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "opposite_lane") {
        return Err(
            RoadMsg::ambiguous_tag(BUSWAY + locale.driving_side.tag(), "opposite_lane").into(),
        );
    }
    if tags.is(BUSWAY + locale.driving_side.opposite().tag(), "lane") {
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            road.push_forward_inside(get_bus(locale, Direction::Forward)?);
        } else {
            // Need an example of this being tagged
            return Err(
                RoadMsg::ambiguous_str("busway:BACKWARD=lane for bidirectional roads").into(),
            );
        }
    }
    if tags.is(
        BUSWAY + locale.driving_side.opposite().tag(),
        "opposite_lane",
    ) {
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            // TODO: does it make sense to have a backward lane on the forward_side????
            road.push_forward_inside(get_bus(locale, Direction::Backward)?);
        } else {
            return Err(RoadMsg::Ambiguous {
                description: None,
                tags: Some(tags.subset(&[
                    BUSWAY + locale.driving_side.opposite().tag(),
                    TagKey::from("oneway"),
                    TagKey::from("oneway:bus"),
                ])),
            }
            .into());
        }
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
pub(in crate::transform::tags_to_lanes) fn lanes_bus(
    tags: &Tags,
    _locale: &Locale,
    _road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    warnings.push(RoadMsg::Unimplemented {
        description: None,
        tags: Some(tags.subset(&[
            LANES + "psv",
            LANES + "psv" + "forward",
            LANES + "psv" + "backward",
            LANES + "psv" + "left",
            LANES + "psv" + "right",
            LANES + "bus",
            LANES + "bus" + "forward",
            LANES + "bus" + "backward",
            LANES + "bus" + "left",
            LANES + "bus" + "right",
        ])),
    });
    Ok(())
}

pub(in crate::transform::tags_to_lanes) fn bus_lanes(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    _warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    match (
        tags.get("bus:lanes"),
        (
            tags.get("bus:lanes:forward"),
            tags.get("bus:lanes:backward"),
        ),
        tags.get("psv:lanes"),
        (
            tags.get("psv:lanes:forward"),
            tags.get("psv:lanes:backward"),
        ),
    ) {
        // lanes:bus or lanes:psv
        (Some(lanes), (None, None), None, (None, None))
        | (None, (None, None), Some(lanes), (None, None)) => {
            let access = Access::split(lanes).map_err(|a| {
                RoadError::from(RoadMsg::Unsupported {
                    description: Some(format!("lanes access '{}'", a)),
                    tags: Some(tags.subset(&["bus:lanes", "psv:lanes"])),
                })
            })?;
            log::trace!("bus:lanes={:?}", access);
            if access.len() != road.len() {
                return Err(RoadMsg::Unsupported {
                    description: Some("lane count mismatch".to_owned()),
                    tags: Some(tags.subset(&[
                        "bus:lanes",
                        "psv:lanes",
                        "lanes",
                        "lanes:forward",
                        "lanes:backward",
                    ])),
                }
                .into());
            }
            for (lane, access) in road.lanes_ltr_mut(locale).zip(access.iter()) {
                if let Access::Designated = access {
                    lane.set_bus(locale)?;
                }
            }
        },
        // lanes:bus:forward and lanes:bus:backward, or lanes:psv:forward and lanes:psv:backward
        (None, (forward, backward), None, (None, None))
        | (None, (None, None), None, (forward, backward)) => {
            if let Some(forward) = forward {
                let forward_access = Access::split(forward).map_err(|a| {
                    RoadError::from(RoadMsg::Unsupported {
                        description: Some(format!("lanes access '{}'", a)),
                        tags: Some(tags.subset(&["bus:lanes:backward", "psv:lanes:backward"])),
                    })
                })?;
                for (lane, access) in road.forward_ltr_mut(locale).zip(forward_access.iter()) {
                    if let Access::Designated = access {
                        lane.set_bus(locale)?;
                    }
                }
            }
            if let Some(backward) = backward {
                let backward_access = Access::split(backward).map_err(|a| {
                    RoadError::from(RoadMsg::Unsupported {
                        description: Some(format!("lanes access '{}'", a)),
                        tags: Some(tags.subset(&["bus:lanes:backward", "psv:lanes:backward"])),
                    })
                })?;
                for (lane, access) in road.backward_ltr_mut(locale).zip(backward_access.iter()) {
                    if let Access::Designated = access {
                        lane.set_bus(locale)?;
                    }
                }
            }
        },
        // Don't try to understand this
        (Some(_), (Some(_), _) | (_, Some(_)), _, _)
        | (Some(_), _, Some(_), _)
        | (Some(_), _, _, (Some(_), _) | (_, Some(_)))
        | (_, (Some(_), _) | (_, Some(_)), _, (Some(_), _) | (_, Some(_)))
        | (_, (Some(_), _) | (_, Some(_)), Some(_), _)
        | (_, _, Some(_), (Some(_), _) | (_, Some(_))) => {
            return Err(RoadMsg::Unsupported {
                description: Some("more than one bus:lanes used".to_owned()),
                tags: Some(tags.subset(&[
                    "bus:lanes",
                    "bus:lanes:forward",
                    "psv:lanes:backward",
                    "psv:lanes",
                    "psv:lanes:forward",
                    "psv:lanes:backward",
                ])),
            }
            .into())
        },
    }

    Ok(())
}
