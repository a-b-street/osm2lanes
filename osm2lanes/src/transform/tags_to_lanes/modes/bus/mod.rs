use crate::locale::Locale;
use crate::road::Designated;
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::access_by_lane::Access;
use crate::transform::tags_to_lanes::{
    Infer, LaneBuilder, LaneBuilderError, Oneway, RoadBuilder, TagsToLanesMsg,
};
use crate::transform::RoadWarnings;

mod busway;
use busway::{busway, Scheme as BuswayScheme};

const LANES: TagKey = TagKey::from("lanes");

impl LaneBuilder {
    #[allow(clippy::unnecessary_wraps)]
    fn set_bus(&mut self, _locale: &Locale) -> Result<(), LaneBuilderError> {
        self.designated = Infer::Direct(Designated::Bus);
        Ok(())
    }
}

impl std::convert::From<LaneBuilderError> for TagsToLanesMsg {
    fn from(error: LaneBuilderError) -> Self {
        TagsToLanesMsg::internal(error.0)
    }
}

#[derive(Debug)]
pub struct BusLaneCount {
    pub forward: usize,
    pub backward: usize,
}

impl BusLaneCount {
    #[allow(clippy::unnecessary_wraps)]
    pub(in crate::transform::tags_to_lanes) fn from_tags(
        tags: &Tags,
        locale: &Locale,
        oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        let busway = BuswayScheme::from_tags(tags, locale, oneway, warnings)?;
        let forward = tags
            .get_parsed("lanes:bus:forward", warnings)
            .unwrap_or_else(|| if busway.forward() { 1 } else { 0 });
        let backward = tags
            .get_parsed("lanes:bus:backward", warnings)
            .unwrap_or_else(|| if busway.backward() { 1 } else { 0 });
        Ok(Self { forward, backward })
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(in crate::transform::tags_to_lanes) fn bus(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), TagsToLanesMsg> {
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
        (false, false, false) => {},
        (true, _, false) => busway(tags, locale, road, warnings)?,
        (false, true, false) => lanes_bus(tags, locale, road, warnings)?,
        (false, false, true) => bus_lanes(tags, locale, road, warnings)?,
        _ => {
            return Err(TagsToLanesMsg::unsupported(
                "more than one bus lanes scheme used",
                tags.subset(&["busway", "lanes:bus", "lanes:psv", "bus:lanes", "psv:lanes"]),
            ))
        },
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn lanes_bus(
    tags: &Tags,
    _locale: &Locale,
    _road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), TagsToLanesMsg> {
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
) -> Result<(), TagsToLanesMsg> {
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
                TagsToLanesMsg::unsupported(
                    &format!("lanes access {}", a),
                    tags.subset(&["bus:lanes", "psv:lanes"]),
                )
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
                ));
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
                    TagsToLanesMsg::unsupported(
                        &format!("lanes access {}", a),
                        tags.subset(&["bus:lanes:backward", "psv:lanes:backward"]),
                    )
                })?;
                for (lane, access) in road.forward_ltr_mut(locale).zip(forward_access.iter()) {
                    if let Access::Designated = access {
                        lane.set_bus(locale)?;
                    }
                }
            }
            if let Some(backward) = backward {
                let backward_access = Access::split(backward).map_err(|a| {
                    TagsToLanesMsg::unsupported(
                        &format!("lanes access {}", a),
                        tags.subset(&["bus:lanes:backward", "psv:lanes:backward"]),
                    )
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
            ))
        },
    }

    Ok(())
}
