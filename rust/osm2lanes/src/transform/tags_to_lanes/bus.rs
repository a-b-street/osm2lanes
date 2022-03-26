use super::{
    Designated, Direction, Infer, LaneBuilder, LaneBuilderError, Locale, ModeResult, RoadBuilder,
    RoadError, RoadMsg, RoadWarnings, TagKey, Tags,
};

const LANES: TagKey = TagKey::from("lanes");

impl RoadError {
    fn unsupported_str(description: &str) -> Self {
        RoadMsg::unsupported_str(description).into()
    }
}

impl LaneBuilder {
    #[allow(clippy::unnecessary_wraps)]
    fn set_bus(&mut self, _locale: &Locale) -> Result<(), LaneBuilderError> {
        self.designated = Infer::Direct(Designated::Bus);
        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(super) fn bus(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> ModeResult {
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
        (false, false, false) => {}
        (true, _, false) => busway(tags, locale, road, warnings)?,
        (false, true, false) => lanes_bus(tags, locale, road, warnings)?,
        (false, false, true) => bus_lanes(tags, locale, road, warnings)?,
        _ => {
            return Err(RoadMsg::Unsupported {
                description: Some("more than one bus lanes scheme used".to_owned()),
                tags: None,
            }
            .into())
        }
    }

    Ok(())
}

fn busway(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    _warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    const BUSWAY: TagKey = TagKey::from("busway");
    if tags.is(BUSWAY, "lane") {
        road.forward_outside_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus(locale)?;
        if !tags.is("oneway", "yes") && !tags.is("oneway:bus", "yes") {
            road.backward_outside_mut()
                .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?
                .set_bus(locale)?;
        }
    }
    if tags.is(BUSWAY, "opposite_lane") {
        road.backward_outside_mut()
            .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?
            .set_bus(locale)?;
    }
    if tags.is(BUSWAY + "both", "lane") {
        road.forward_outside_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus(locale)?;
        road.backward_outside_mut()
            .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?
            .set_bus(locale)?;
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            return Err(RoadMsg::ambiguous_str("busway:both=lane for oneway roads").into());
        }
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "lane") {
        road.forward_outside_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus(locale)?;
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "opposite_lane") {
        return Err(
            RoadMsg::ambiguous_tag(BUSWAY + locale.driving_side.tag(), "opposite_lane").into(),
        );
    }
    if tags.is(BUSWAY + locale.driving_side.opposite().tag(), "lane") {
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            road.forward_inside_mut()
                .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
                .set_bus(locale)?;
        } else {
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
            let lane = road
                .forward_inside_mut()
                .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?;
            lane.set_bus(locale)?;
            lane.direction = Infer::Direct(Direction::Backward);
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
fn lanes_bus(
    tags: &Tags,
    _locale: &Locale,
    _road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> ModeResult {
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

#[derive(Debug)]
enum Access {
    None,
    No,
    Yes,
    Designated,
}

impl std::str::FromStr for Access {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Self::None),
            "no" => Ok(Self::No),
            "yes" => Ok(Self::Yes),
            "designated" => Ok(Self::Designated),
            _ => Err(s.to_owned()),
        }
    }
}

fn split_access(lanes: &str) -> Result<Vec<Access>, String> {
    lanes.split('|').map(str::parse).collect()
}

fn bus_lanes(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    _warnings: &mut RoadWarnings,
) -> ModeResult {
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
            let access = split_access(lanes).map_err(|a| {
                RoadError::from(RoadMsg::Unsupported {
                    description: Some(format!("lanes access {}", a)),
                    tags: Some(tags.subset(&["bus:lanes", "psv:lanes"])),
                })
            })?;
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
        }
        // lanes:bus:forward and lanes:bus:backward, or lanes:psv:forward and lanes:psv:backward
        (None, (forward, backward), None, (None, None))
        | (None, (None, None), None, (forward, backward)) => {
            if let Some(forward) = forward {
                let forward_access = split_access(forward).map_err(|a| {
                    RoadError::from(RoadMsg::Unsupported {
                        description: Some(format!("lanes access {}", a)),
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
                let backward_access = split_access(backward).map_err(|a| {
                    RoadError::from(RoadMsg::Unsupported {
                        description: Some(format!("lanes access {}", a)),
                        tags: Some(tags.subset(&["bus:lanes:backward", "psv:lanes:backward"])),
                    })
                })?;
                for (lane, access) in road.backward_ltr_mut(locale).zip(backward_access.iter()) {
                    if let Access::Designated = access {
                        lane.set_bus(locale)?;
                    }
                }
            }
        }
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
        }
    }

    Ok(())
}
