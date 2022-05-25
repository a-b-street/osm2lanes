use crate::locale::Locale;
use crate::road::Direction;
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::{Infer, Oneway, RoadBuilder};
use crate::transform::{RoadWarnings, TagsToLanesMsg};

const BUSWAY: TagKey = TagKey::from_static("busway");
const ONEWAY: TagKey = TagKey::from_static("oneway");

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

fn get_bus_lane<T>(tags: &Tags, key: T, warnings: &mut RoadWarnings) -> Lane
where
    T: AsRef<str>,
    TagKey: From<T>,
{
    match tags.get(&key) {
        None => Lane::None,
        Some("lane") => Lane::Lane,
        Some("opposite_lane") => Lane::Opposite,
        Some(v) => {
            warnings.push(TagsToLanesMsg::unsupported_tag(key, v));
            Lane::None
        },
    }
}

impl Scheme {
    #[allow(clippy::unnecessary_wraps)]
    pub(in crate::transform::tags_to_lanes) fn from_tags(
        tags: &Tags,
        road_oneway: Oneway,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        let bus_oneway: Oneway = match tags.get(ONEWAY + "bus") {
            Some("yes") => Oneway::Yes,
            Some("no") => Oneway::No,
            None => road_oneway,
            Some(v) => {
                warnings.push(TagsToLanesMsg::unsupported_tag(ONEWAY + "bus", v));
                road_oneway
            },
        };

        let busway_root: Lane = get_bus_lane(tags, BUSWAY, warnings);
        let busway_root: Variant = match (busway_root, bus_oneway) {
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

        let busway_both: Lane = get_bus_lane(tags, BUSWAY + "both", warnings);
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
        let busway_forward: Lane = get_bus_lane(tags, busway_forward_key(), warnings);
        if let Lane::Opposite = busway_forward {
            warnings.push(TagsToLanesMsg::unsupported_tags(
                tags.subset(&[busway_forward_key()]),
            ));
        }
        let busway_backward_key = || BUSWAY + locale.driving_side.opposite().tag();
        let busway_backward: Lane = get_bus_lane(tags, busway_backward_key(), warnings);
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
        } else if let Variant::Both | Variant::Forward | Variant::Backward = busway_forward_backward
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

pub(in crate::transform::tags_to_lanes) fn apply_busway(
    road: &mut RoadBuilder,
    scheme: &Scheme,
    locale: &Locale,
) -> Result<(), TagsToLanesMsg> {
    if let Variant::Forward | Variant::Both = scheme.0 {
        road.forward_outside_mut()
            .ok_or_else(|| TagsToLanesMsg::unsupported_str("no forward lanes for busway"))?
            .set_bus(locale)?;
    }
    if let Variant::Backward | Variant::Both = scheme.0 {
        if let Some(backward_outside) = road.backward_outside_mut() {
            backward_outside.set_bus(locale)?;
        } else {
            let forward_inside = road
                .forward_inside_mut()
                .ok_or_else(|| TagsToLanesMsg::unsupported_str("no forward lanes for busway"))?;
            forward_inside.set_bus(locale)?;
            forward_inside.direction = Infer::Direct(Direction::Backward);
        }
    }
    Ok(())
}
