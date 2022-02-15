use super::*;

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
            direction: Infer::Direct(LaneDirection::Forward),
            designated: Infer::Direct(LaneDesignated::Bicycle),
            ..Default::default()
        }
    }
    fn cycle_backward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(LaneDirection::Backward),
            designated: Infer::Direct(LaneDesignated::Bicycle),
            ..Default::default()
        }
    }
    fn cycle_both(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(LaneDirection::Both),
            designated: Infer::Direct(LaneDesignated::Bicycle),
            ..Default::default()
        }
    }
}

pub(super) fn bicycle(
    tags: &Tags,
    locale: &Locale,
    oneway: Oneway,
    forward_side: &mut Vec<LaneBuilder>,
    backward_side: &mut Vec<LaneBuilder>,
    warnings: &mut RoadWarnings,
) -> ModeResult {
    if tags.is_cycleway(None) {
        if tags.is_cycleway(Some(WaySide::Both))
            || tags.is_cycleway(Some(WaySide::Right))
            || tags.is_cycleway(Some(WaySide::Left))
        {
            return Err(RoadMsg::unsupported_str("cycleway=* with any cycleway:* values").into());
        }
        forward_side.push(LaneBuilder::cycle_forward(locale));
        if oneway.into() {
            if !backward_side.is_empty() {
                // TODO safety check to be checked
                warnings.push(RoadMsg::Unimplemented {
                    description: Some(
                        "oneway has backwards lanes when adding cycleways".to_owned(),
                    ),
                    tags: Some(tags.subset(&["oneway", "cycleway"])),
                })
            }
        } else {
            backward_side.push(LaneBuilder::cycle_backward(locale));
        }
    } else if tags.is_cycleway(Some(WaySide::Both)) {
        forward_side.push(LaneBuilder::cycle_forward(locale));
        backward_side.push(LaneBuilder::cycle_backward(locale));
    } else {
        // cycleway=opposite_lane
        if tags.is(CYCLEWAY, "opposite_lane") {
            warnings.push(RoadMsg::Deprecated {
                deprecated_tags: tags.subset(&["cycleway", "oneway"]),
                suggested_tags: None,
            });
            backward_side.push(LaneBuilder::cycle_backward(locale));
        }
        // cycleway=opposite oneway=yes oneway:bicycle=no
        if tags.is(CYCLEWAY, "opposite") {
            if !(oneway.into() && tags.is("oneway:bicycle", "no")) {
                return Err(RoadMsg::unsupported_str(
                    "cycleway=opposite without oneway=yes oneway:bicycle=no",
                )
                .into());
            }
            backward_side.push(LaneBuilder::cycle_backward(locale));
        }
        // cycleway:FORWARD=*
        if tags.is_cycleway(Some(locale.driving_side.into())) {
            if tags.is(CYCLEWAY + locale.driving_side.tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
            {
                forward_side.push(LaneBuilder::cycle_both(locale));
            } else {
                forward_side.push(LaneBuilder::cycle_forward(locale));
            }
        }
        // cycleway:FORWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            warnings.push(RoadMsg::Deprecated {
                deprecated_tags: tags.subset(&[CYCLEWAY + locale.driving_side.tag()]),
                suggested_tags: None,
            });
            forward_side.push(LaneBuilder::cycle_backward(locale));
        }
        // cycleway:BACKWARD=*
        if tags.is_cycleway(Some(locale.driving_side.opposite().into())) {
            if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "yes",
            ) {
                forward_side.insert(0, LaneBuilder::cycle_forward(locale));
            } else if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "-1",
            ) {
                backward_side.push(LaneBuilder::cycle_backward(locale));
            } else if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "no",
            ) || tags.is("oneway:bicycle", "no")
            {
                backward_side.push(LaneBuilder::cycle_both(locale));
            } else if oneway.into() {
                // A oneway road with a cycleway on the wrong side
                forward_side.insert(0, LaneBuilder::cycle_forward(locale));
            } else {
                // A contraflow bicycle lane
                backward_side.push(LaneBuilder::cycle_backward(locale));
            }
        }
        // cycleway:BACKWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.opposite().tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            return Err(RoadMsg::Unsupported {
                description: None,
                tags: Some(tags.subset(&[CYCLEWAY + locale.driving_side.opposite().tag()])),
            }
            .into());
        }
    }
    Ok(())
}
