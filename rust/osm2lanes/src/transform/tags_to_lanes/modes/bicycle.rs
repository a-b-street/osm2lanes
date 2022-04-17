use crate::locale::Locale;
use crate::road::{Designated, Direction};
use crate::tag::Tags;
use crate::transform::tags::CYCLEWAY;
use crate::transform::tags_to_lanes::{Infer, LaneBuilder, LaneType, RoadBuilder, TagsToLanesMsg};
use crate::transform::{RoadError, RoadWarnings, WaySide};

impl Tags {
    fn is_cycleway(&self, side: Option<WaySide>) -> bool {
        if let Some(side) = side {
            self.is_any(CYCLEWAY + side.as_str(), &["lane", "track"])
        } else {
            self.is_any(CYCLEWAY, &["lane", "track"])
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
) -> Result<(), RoadError> {
    if tags.is_cycleway(None) {
        if tags.is_cycleway(Some(WaySide::Both))
            || tags.is_cycleway(Some(WaySide::Right))
            || tags.is_cycleway(Some(WaySide::Left))
        {
            return Err(
                TagsToLanesMsg::unsupported_str("cycleway=* with any cycleway:* values").into(),
            );
        }
        road.push_forward_outside(LaneBuilder::cycle_forward(locale));
        if road.oneway.into() {
            if road.backward_outside().is_some() {
                // TODO validity of this safety check
                warnings.push(TagsToLanesMsg::unimplemented(
                    "oneway has backwards lanes when adding cycleways",
                    tags.subset(&["oneway", "cycleway"]),
                ));
            }
        } else {
            road.push_backward_outside(LaneBuilder::cycle_backward(locale));
        }
    } else if tags.is_cycleway(Some(WaySide::Both)) {
        road.push_forward_outside(LaneBuilder::cycle_forward(locale));
        road.push_backward_outside(LaneBuilder::cycle_backward(locale));
    } else {
        // cycleway=opposite_lane
        if tags.is(CYCLEWAY, "opposite_lane") {
            warnings.push(TagsToLanesMsg::deprecated_tags(
                tags.subset(&["cycleway", "oneway"]),
            ));
            road.push_backward_outside(LaneBuilder::cycle_backward(locale));
        }
        // cycleway=opposite oneway=yes oneway:bicycle=no
        if tags.is(CYCLEWAY, "opposite") {
            if !(road.oneway.into() && tags.is("oneway:bicycle", "no")) {
                return Err(TagsToLanesMsg::unsupported_str(
                    "cycleway=opposite without oneway=yes oneway:bicycle=no",
                )
                .into());
            }
            road.push_backward_outside(LaneBuilder::cycle_backward(locale));
        }
        // cycleway:FORWARD=*
        if tags.is_cycleway(Some(locale.driving_side.into())) {
            if tags.is(CYCLEWAY + locale.driving_side.tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
            {
                road.push_forward_outside(LaneBuilder::cycle_both(locale));
            } else {
                road.push_forward_outside(LaneBuilder::cycle_forward(locale));
            }
        }
        // cycleway:FORWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            warnings.push(TagsToLanesMsg::deprecated_tags(
                tags.subset(&[CYCLEWAY + locale.driving_side.tag()]),
            ));
            road.push_forward_outside(LaneBuilder::cycle_backward(locale));
        }
        // cycleway:BACKWARD=*
        if tags.is_cycleway(Some(locale.driving_side.opposite().into())) {
            if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "yes",
            ) {
                road.push_forward_inside(LaneBuilder::cycle_forward(locale));
            } else if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "-1",
            ) {
                road.push_backward_outside(LaneBuilder::cycle_backward(locale));
            } else if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "no",
            ) || tags.is("oneway:bicycle", "no")
            {
                road.push_backward_outside(LaneBuilder::cycle_both(locale));
            } else if road.oneway.into() {
                // A oneway road with a cycleway on the wrong side
                road.push_forward_inside(LaneBuilder::cycle_forward(locale));
            } else {
                // A contraflow bicycle lane
                road.push_backward_outside(LaneBuilder::cycle_backward(locale));
            }
        }
        // cycleway:BACKWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.opposite().tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            return Err(TagsToLanesMsg::unsupported_tags(
                tags.subset(&[CYCLEWAY + locale.driving_side.opposite().tag()]),
            )
            .into());
        }
    }
    Ok(())
}
