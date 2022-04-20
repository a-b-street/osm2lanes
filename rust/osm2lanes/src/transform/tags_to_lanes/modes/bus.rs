use crate::locale::Locale;
use crate::road::{Designated, Direction};
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::access_by_lane::Access;
use crate::transform::tags_to_lanes::{
    Infer, LaneBuilder, LaneBuilderError, Oneway, RoadBuilder, TagsToLanesMsg,
};
use crate::transform::{RoadError, RoadWarnings};

const LANES: TagKey = TagKey::from("lanes");

impl RoadError {
    fn unsupported_str(description: &str) -> Self {
        TagsToLanesMsg::unsupported_str(description).into()
    }
}

impl LaneBuilder {
    #[allow(clippy::unnecessary_wraps)]
    fn set_bus(&mut self, _locale: &Locale) -> Result<(), LaneBuilderError> {
        self.designated = Infer::Direct(Designated::Bus);
        Ok(())
    }
}

const BUSWAY: TagKey = TagKey::from("busway");

#[derive(Debug)]
pub struct BuswayScheme {
    pub forward_side_direction: Infer<Option<Direction>>,
    pub backward_side_direction: Infer<Option<Direction>>,
}

#[derive(Debug)]
pub struct LanesBusScheme {
    pub forward: Infer<usize>,
    pub backward: Infer<usize>,
    pub bothways: Infer<usize>,
}

#[allow(clippy::unnecessary_wraps)]
pub(in crate::transform::tags_to_lanes) fn bus(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    // https://wiki.openstreetmap.org/wiki/Bus_lanes
    // 3 schemes, for simplicity we only allow one at a time
    match (
        tags.tree().get("busway").is_some(),
        tags.tree()
            .get("lanes:bus")
            .or_else(|| tags.tree().get("lanes:psv"))
            .is_some(),
        tags.tree()
            .get("bus:lanes")
            .or_else(|| tags.tree().get("psv:lanes"))
            .is_some(),
    ) {
        (false, false, false) | (true, _, false) => {},
        (false, true, false) => lanes_bus(tags, locale, road, warnings)?,
        (false, false, true) => bus_lanes(tags, locale, road, warnings)?,
        _ => {
            return Err(TagsToLanesMsg::unsupported(
                "more than one bus lanes scheme used",
                tags.subset(&["busway", "lanes:bus", "lanes:psv", "bus:lanes", "psv:lanes"]),
            )
            .into())
        },
    }

    Ok(())
}

impl BuswayScheme {
    pub(in crate::transform::tags_to_lanes) fn new(
        tags: &Tags,
        locale: &Locale,
        oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Self {
        let mut busway = Self {
            forward_side_direction: Infer::Default(None),
            backward_side_direction: Infer::Default(None),
        };

        // TODO oneway:bus=no on a oneway road (w/ busway=lane) implies 2 bus lanes.
        // TODO I think this logic can be simplified using the match style.

        if tags.is(BUSWAY, "lane") {
            busway.forward_side_direction = Infer::Direct(Some(Direction::Forward));
            if oneway == Oneway::No && !tags.is("oneway:bus", "yes") {
                busway.backward_side_direction = Infer::Direct(Some(Direction::Backward));
            }
        }
        if tags.is(BUSWAY, "opposite_lane") {
            busway.backward_side_direction = Infer::Direct(Some(Direction::Backward));
        }
        if tags.is(BUSWAY + "both", "lane") {
            busway.forward_side_direction = Infer::Direct(Some(Direction::Forward));
            busway.backward_side_direction = Infer::Direct(Some(Direction::Backward));
            if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
                warnings.push(TagsToLanesMsg::ambiguous_str(
                    "busway:both=lane for oneway roads",
                ));
            }
        }
        if tags.is(BUSWAY + locale.driving_side.tag(), "lane") {
            busway.forward_side_direction = Infer::Direct(Some(Direction::Forward));
        }
        if tags.is(BUSWAY + locale.driving_side.tag(), "opposite_lane") {
            warnings.push(TagsToLanesMsg::ambiguous_tag(
                BUSWAY + locale.driving_side.tag(),
                "opposite_lane",
            ));
        }
        if tags.is(BUSWAY + locale.driving_side.opposite().tag(), "lane") {
            if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
                busway.forward_side_direction = Infer::Direct(Some(Direction::Forward));
            } else {
                warnings.push(TagsToLanesMsg::ambiguous_str(
                    "busway:BACKWARD=lane for bidirectional roads",
                ));
            }
        }
        if tags.is(
            BUSWAY + locale.driving_side.opposite().tag(),
            "opposite_lane",
        ) {
            if tags.is("oneway", "yes") {
                busway.backward_side_direction = Infer::Direct(Some(Direction::Backward));
            }

            if tags.is("oneway:bus", "yes") {
                // TODO ignore the busway tag, or ignore oneway:bus=yes?
                warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                    BUSWAY + locale.driving_side.opposite().tag(),
                    TagKey::from("oneway:bus"),
                ])));
            }
        }

        busway
    }
}

impl RoadBuilder {
    pub fn set_busway_scheme(
        &mut self,
        oneway: &Oneway,
        busway: &BuswayScheme,
        locale: &Locale,
        _warnings: &mut RoadWarnings,
    ) -> Result<(), RoadError> {
        if let Some(Some(d)) = busway.forward_side_direction.some() {
            let lane = self
                .forward_outside_mut()
                .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?;
            lane.set_bus(locale)?;
            lane.direction = Infer::Direct(d);
        }
        match (oneway, busway.backward_side_direction.some()) {
            (Oneway::Yes, Some(Some(Direction::Backward))) => {
                // Tagged oneway, but there *is* a backward (bus) lane.
                self.backward_outside_mut()
                    .ok_or_else(|| {
                        RoadError::unsupported_str("no backward lanes for contraflow busway")
                    })?
                    .set_bus(locale)?;
            },
            (Oneway::Yes, Some(Some(Direction::Forward))) => {
                // Busway on the "inside" side of the oneway road.
                self.forward_inside_mut()
                    .ok_or_else(|| {
                        RoadError::unsupported_str("no forward lanes for backward side busway")
                    })?
                    .set_bus(locale)?;
            },
            (Oneway::No, Some(Some(d))) => {
                let lane = self
                    .backward_outside_mut()
                    .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?;
                lane.set_bus(locale)?;
                lane.direction = Infer::Direct(d);
            },
            _ => {},
        }
        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn lanes_bus(
    tags: &Tags,
    _locale: &Locale,
    _road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    warnings.push(TagsToLanesMsg::unimplemented_tags(tags.subset(&[
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
    ])));
    Ok(())
}

fn bus_lanes(
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
                RoadError::Msg(TagsToLanesMsg::unsupported(
                    &format!("lanes access {}", a),
                    tags.subset(&["bus:lanes", "psv:lanes"]),
                ))
            })?;
            if access.len() != road.len() {
                return Err(TagsToLanesMsg::unsupported(
                    "lane count mismatch",
                    tags.subset(&[
                        "bus:lanes",
                        "psv:lanes",
                        "lanes",
                        "lanes:forward",
                        "lanes:backward",
                    ]),
                )
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
                    RoadError::Msg(TagsToLanesMsg::unsupported(
                        &format!("lanes access {}", a),
                        tags.subset(&["bus:lanes:backward", "psv:lanes:backward"]),
                    ))
                })?;
                for (lane, access) in road.forward_ltr_mut(locale).zip(forward_access.iter()) {
                    if let Access::Designated = access {
                        lane.set_bus(locale)?;
                    }
                }
            }
            if let Some(backward) = backward {
                let backward_access = Access::split(backward).map_err(|a| {
                    RoadError::Msg(TagsToLanesMsg::unsupported(
                        &format!("lanes access {}", a),
                        tags.subset(&["bus:lanes:backward", "psv:lanes:backward"]),
                    ))
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
            return Err(TagsToLanesMsg::unsupported(
                "more than one bus:lanes used",
                tags.subset(&[
                    "bus:lanes",
                    "bus:lanes:forward",
                    "psv:lanes:backward",
                    "psv:lanes",
                    "psv:lanes:forward",
                    "psv:lanes:backward",
                ]),
            )
            .into())
        },
    }

    Ok(())
}
