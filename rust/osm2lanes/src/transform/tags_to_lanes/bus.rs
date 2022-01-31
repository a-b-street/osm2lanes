use super::*;

const LANES: TagKey = TagKey::from("lanes");

impl RoadError {
    fn unsupported_str(description: &str) -> Self {
        RoadMsg::unsupported_str(description).into()
    }
}

pub fn bus(
    tags: &Tags,
    locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) -> ModeResult {
    // https://wiki.openstreetmap.org/wiki/Bus_lanes
    // 3 schemes, for simplicity we only allow one at a time
    let tag_tree = tags.tree();

    match (
        tag_tree.get("busway").is_some(),
        tag_tree
            .get("lanes:bus")
            .or_else(|| tag_tree.get("lanes:psv"))
            .is_some(),
        tag_tree
            .get("bus:lanes")
            .or_else(|| tag_tree.get("psv:lanes"))
            .is_some(),
    ) {
        (false, false, false) => {}
        (true, _, false) => busway(tags, locale, oneway, forward_side, backward_side)?,
        (false, true, false) => lanes_bus(tags, locale, oneway, forward_side, backward_side)?,
        (false, false, true) => bus_lanes(tags, locale, oneway, forward_side, backward_side)?,
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
    _oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) -> ModeResult {
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
            return Err(RoadError::ambiguous_str(
                "busway:both=lane for oneway roads",
            ));
        }
    }
    if tags.is(BUSWAY + locale.driving_side.tag(), "lane") {
        forward_side
            .last_mut()
            .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
            .set_bus()?;
    }
    if tags.is(BUSWAY + locale.driving_side.opposite().tag(), "lane") {
        if tags.is("oneway", "yes") || tags.is("oneway:bus", "yes") {
            forward_side
                .first_mut()
                .ok_or_else(|| RoadError::unsupported_str("no forward lanes for busway"))?
                .set_bus()?;
        } else {
            return Err(RoadError::ambiguous_str(
                "busway:BACKWARD=lane for bidirectional roads",
            ));
        }
    }
    Ok(())
}

fn lanes_bus(
    tags: &Tags,
    _locale: &Locale,
    _oneway: bool,
    _forward_side: &mut Vec<Lane>,
    _backward_side: &mut Vec<Lane>,
) -> ModeResult {
    return Err(RoadMsg::Unimplemented {
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
    }
    .into());
}

fn bus_lanes(
    tags: &Tags,
    _locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) -> ModeResult {
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

    Ok(())
}
