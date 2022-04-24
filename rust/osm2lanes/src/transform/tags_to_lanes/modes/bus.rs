use crate::locale::Locale;
use crate::road::Designated;
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::access_by_lane::Access;
use crate::transform::tags_to_lanes::{
    Infer, LaneBuilder, LaneBuilderError, Oneway, RoadBuilder, TagsToLanesMsg,
};
use crate::transform::{RoadError, RoadWarnings};

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
pub struct BusLanesCount {
    pub forward: usize,
    pub backward: usize,
}

impl BusLanesCount {
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
        (false, false, false) => {},
        (true, _, false) => busway(tags, locale, road, warnings)?,
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

mod busway {
    use crate::locale::Locale;
    use crate::road::Direction;
    use crate::tag::{TagKey, Tags};
    use crate::transform::tags_to_lanes::{Infer, Oneway, RoadBuilder};
    use crate::transform::{RoadWarnings, TagsToLanesMsg};

    const BUSWAY: TagKey = TagKey::from("busway");
    const ONEWAY: TagKey = TagKey::from("oneway");

    #[derive(Debug, PartialEq)]
    pub(in crate::transform::tags_to_lanes) enum Variant {
        None,
        Forward,
        Backward,
        Both,
    }

    /// Inferred busway scheme for forward lane and backward lane existing
    #[derive(Debug)]
    pub(in crate::transform::tags_to_lanes) struct Scheme(Variant);

    impl Scheme {
        pub fn forward(&self) -> bool {
            match self.0 {
                Variant::None | Variant::Backward => false,
                Variant::Forward | Variant::Both => true,
            }
        }

        pub fn backward(&self) -> bool {
            match self.0 {
                Variant::None | Variant::Forward => false,
                Variant::Backward | Variant::Both => true,
            }
        }
    }

    enum Lane {
        None,
        Lane,
        Opposite,
    }

    impl Tags {
        fn get_bus_lane<T>(&self, key: T, warnings: &mut RoadWarnings) -> Lane
        where
            T: AsRef<str>,
            TagKey: From<T>,
        {
            match self.get(&key) {
                None => Lane::None,
                Some("lane") => Lane::Lane,
                Some("opposite_lane") => Lane::Opposite,
                Some(v) => {
                    warnings.push(TagsToLanesMsg::unsupported_tag(key, v));
                    Lane::None
                },
            }
        }
    }

    impl Scheme {
        #[allow(clippy::unnecessary_wraps)]
        pub(in crate::transform::tags_to_lanes) fn from_tags(
            tags: &Tags,
            locale: &Locale,
            oneway: Oneway,
            warnings: &mut RoadWarnings,
        ) -> Result<Self, TagsToLanesMsg> {
            let oneway: Oneway = match tags.get(ONEWAY + "bus") {
                Some("yes") => Oneway::Yes,
                Some("no") => Oneway::No,
                None => oneway,
                Some(v) => {
                    warnings.push(TagsToLanesMsg::unsupported_tag(ONEWAY + "bus", v));
                    oneway
                },
            };

            let busway_root: Lane = tags.get_bus_lane(BUSWAY, warnings);
            let busway_root: Variant = match (busway_root, oneway) {
                (Lane::None, _) => Variant::None,
                (Lane::Lane, Oneway::No) => Variant::Both,
                (Lane::Lane, Oneway::Yes) => Variant::Forward,
                (Lane::Opposite, Oneway::No) => {
                    warnings.push(TagsToLanesMsg::unsupported_tags(tags.subset(&[
                        BUSWAY,
                        ONEWAY,
                        ONEWAY + "bus",
                    ])));
                    Variant::None
                },
                (Lane::Opposite, Oneway::Yes) => Variant::Backward,
            };

            let busway_both: Lane = tags.get_bus_lane(BUSWAY + "both", warnings);
            let busway_both: Variant = match busway_both {
                Lane::None => Variant::None,
                Lane::Lane => Variant::Both,
                Lane::Opposite => {
                    warnings.push(TagsToLanesMsg::unsupported_tags(
                        tags.subset(&[BUSWAY + "both"]),
                    ));
                    Variant::None
                },
            };

            let busway_forward_key = || BUSWAY + locale.driving_side.tag();
            let busway_forward: Lane = tags.get_bus_lane(busway_forward_key(), warnings);
            if let Lane::Opposite = busway_forward {
                warnings.push(TagsToLanesMsg::unsupported_tags(
                    tags.subset(&[busway_forward_key()]),
                ));
            }
            let busway_backward_key = || BUSWAY + locale.driving_side.opposite().tag();
            let busway_backward: Lane = tags.get_bus_lane(busway_backward_key(), warnings);
            let busway_forward_backward = match (busway_forward, busway_backward) {
                (Lane::None | Lane::Opposite, Lane::None) => Variant::None,
                (Lane::Lane, Lane::None) => Variant::Forward,
                (Lane::None | Lane::Opposite, Lane::Lane | Lane::Opposite) => Variant::Backward,
                (Lane::Lane, Lane::Lane | Lane::Opposite) => Variant::Both,
            };

            if let Variant::Both = busway_both {
                if let Variant::Forward | Variant::Backward = busway_forward_backward {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                        BUSWAY + "both",
                        busway_forward_key(),
                        busway_backward_key(),
                    ])));
                }
                if let Variant::Forward | Variant::Backward = busway_root {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                        BUSWAY,
                        ONEWAY,
                        ONEWAY + "bus",
                        BUSWAY + "both",
                    ])));
                }
                Ok(Scheme(Variant::Both))
            } else if let Variant::Both | Variant::Forward | Variant::Backward =
                busway_forward_backward
            {
                if busway_root != Variant::None && busway_root != busway_forward_backward {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                        BUSWAY,
                        ONEWAY,
                        ONEWAY + "bus",
                        busway_forward_key(),
                        busway_backward_key(),
                    ])));
                }
                Ok(Scheme(busway_forward_backward))
            } else {
                Ok(Scheme(busway_root))
            }
        }
    }

    pub(in crate::transform::tags_to_lanes) fn busway(
        tags: &Tags,
        locale: &Locale,
        road: &mut RoadBuilder,
        warnings: &mut RoadWarnings,
    ) -> Result<(), TagsToLanesMsg> {
        let scheme = Scheme::from_tags(tags, locale, road.oneway, warnings)?;

        if let Variant::Forward | Variant::Both = scheme.0 {
            road.forward_outside_mut()
                .ok_or_else(|| TagsToLanesMsg::unsupported_str("no forward lanes for busway"))?
                .set_bus(locale)?;
        }

        if let Variant::Backward | Variant::Both = scheme.0 {
            if let Some(backward_outside) = road.backward_outside_mut() {
                backward_outside.set_bus(locale)?;
            } else {
                let forward_inside = road.forward_inside_mut().ok_or_else(|| {
                    TagsToLanesMsg::unsupported_str("no forward lanes for busway")
                })?;
                forward_inside.set_bus(locale)?;
                forward_inside.direction = Infer::Direct(Direction::Backward);
            }
        }
        Ok(())
    }
}
use busway::{busway, Scheme as BuswayScheme};

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
                RoadError::from(TagsToLanesMsg::unsupported(
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
                    RoadError::from(TagsToLanesMsg::unsupported(
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
                    RoadError::from(TagsToLanesMsg::unsupported(
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
