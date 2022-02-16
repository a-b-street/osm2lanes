use super::*;

const LANES: TagKey = TagKey::from("lanes");

impl RoadError {
    fn unsupported_str(description: &str) -> Self {
        RoadMsg::unsupported_str(description).into()
    }
}

impl LaneBuilder {
    fn set_bus(&mut self) -> Result<(), LaneBuilderError> {
        self.designated = Infer::Direct(LaneDesignated::Bus);
        Ok(())
    }
}

pub(super) fn bus(
    tags: &Tags,
    locale: &Locale,
    oneway: Oneway,
    forward_side: &mut [LaneBuilder],
    backward_side: &mut [LaneBuilder],
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
        (true, _, false) => busway(tags, locale, oneway, forward_side, backward_side, warnings)?,
        (false, true, false) => {
            lanes_bus(tags, locale, oneway, forward_side, backward_side, warnings)?
        }
        (false, false, true) => {
            bus_lanes(tags, locale, oneway, forward_side, backward_side, warnings)?
        }
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
    _oneway: Oneway,
    forward_side: &mut [LaneBuilder],
    backward_side: &mut [LaneBuilder],
    _warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    const BUSWAY: TagKey = TagKey::from("busway");
    if tags.is(BUSWAY, "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus()?;
        if !tags.is("oneway", "yes") && !tags.is("oneway:bus", "yes") {
            backward_side
                .last_mut()
                .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?
                .set_bus()?;
        }
    }
    if tags.is(BUSWAY, "opposite_lane") {
        backward_side
            .last_mut()
            .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?
            .set_bus()?;
    }
    if tags.is(BUSWAY + "both", "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus()?;
        backward_side
            .last_mut()
            .ok_or_else(|| RoadError::unsupported_str("no backward lanes for busway"))?
            .set_bus()?;
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            return Err(RoadMsg::ambiguous_str("busway:both=lane for oneway roads").into());
        }
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus()?;
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "opposite_lane") {
        return Err(
            RoadMsg::ambiguous_tag(BUSWAY + locale.driving_side.tag(), "opposite_lane").into(),
        );
    }
    if tags.is(BUSWAY + locale.driving_side.opposite().tag(), "lane") {
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            forward_side
                .first_mut()
                .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
                .set_bus()?;
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
            let lane = forward_side
                .first_mut()
                .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?;
            lane.set_bus()?;
            lane.direction = Infer::Direct(LaneDirection::Backward);
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

fn lanes_bus(
    tags: &Tags,
    _locale: &Locale,
    _oneway: Oneway,
    _forward_side: &mut [LaneBuilder],
    _backward_side: &mut [LaneBuilder],
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
    lanes.split('|').map(|s| s.parse()).collect()
}

fn bus_lanes(
    tags: &Tags,
    locale: &Locale,
    _oneway: Oneway,
    forward_lanes: &mut [LaneBuilder],
    backward_lanes: &mut [LaneBuilder],
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
            if access.len() != forward_lanes.len() + backward_lanes.len() {
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
            // TODO: maybe have a `RoadBuilder` with `forward`, `backward`, and `lanes`?
            let lanes = match locale.driving_side {
                DrivingSide::Left => forward_lanes
                    .iter_mut()
                    .rev()
                    .chain(backward_lanes.iter_mut()),
                DrivingSide::Right => backward_lanes
                    .iter_mut()
                    .rev()
                    .chain(forward_lanes.iter_mut()),
            };
            for (lane, access) in lanes.zip(access.iter()) {
                match access {
                    Access::None => {}
                    Access::No => {}
                    Access::Yes => {}
                    Access::Designated => lane.set_bus()?,
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
                for (lane, access) in forward_lanes.iter_mut().zip(forward_access.iter()) {
                    match access {
                        Access::None => {}
                        Access::No => {}
                        Access::Yes => {}
                        Access::Designated => lane.set_bus()?,
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
                for (lane, access) in backward_lanes.iter_mut().zip(backward_access.iter()) {
                    match access {
                        Access::None => {}
                        Access::No => {}
                        Access::Yes => {}
                        Access::Designated => lane.set_bus()?,
                    }
                }
            }
        }
        _ => todo!(),
    }

    Ok(())
}
