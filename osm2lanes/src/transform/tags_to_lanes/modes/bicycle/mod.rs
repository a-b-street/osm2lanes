use osm_tag_schemes::Access;
use osm_tags::Tags;

use crate::locale::Locale;
use crate::road::{AccessAndDirection, Designated, Direction};
use crate::transform::tags_to_lanes::road::LaneType;
use crate::transform::tags_to_lanes::{Infer, LaneBuilder, RoadBuilder, TagsToLanesMsg};
use crate::transform::RoadWarnings;

pub(in crate::transform::tags_to_lanes) mod cycleway;

impl LaneBuilder {
    fn cycle(way: cycleway::Way) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(way.direction),
            designated: Infer::Direct(Designated::Bicycle),
            width: way.width.unwrap_or_default(),
            cycleway_variant: Some(way.variant),
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
    let scheme = cycleway::Scheme::from_tags(tags, locale, road.oneway, warnings)?;
    log::trace!("cycleway scheme: {scheme:?}");
    match scheme.location {
        cycleway::Location::None => {},
        cycleway::Location::Forward(way) => {
            if let cycleway::Variant::Lane | cycleway::Variant::Track = way.variant {
                road.push_forward_outside(LaneBuilder::cycle(way));
            }
            // TODO: Do nothing if forward sharing the lane? What if we are on a bus-only road?
        },
        cycleway::Location::Backward(way) => match way.variant {
            cycleway::Variant::Lane | cycleway::Variant::Track => {
                road.push_backward_outside(LaneBuilder::cycle(way))
            },
            cycleway::Variant::SharedMotor => {
                road.forward_outside_mut()
                    .ok_or_else(|| {
                        TagsToLanesMsg::unsupported_str("no forward lanes for cycleway")
                    })?
                    .access
                    .bicycle = Infer::Direct(AccessAndDirection {
                    access: Access::Yes,
                    direction: Some(Direction::Both),
                });
            },
        },
        cycleway::Location::Both { forward, backward } => {
            road.push_forward_outside(LaneBuilder::cycle(forward));
            road.push_backward_outside(LaneBuilder::cycle(backward));
        },
    }
    Ok(())
}
