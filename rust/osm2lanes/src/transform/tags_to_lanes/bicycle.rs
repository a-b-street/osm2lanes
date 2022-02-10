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

pub fn bicycle(
    tags: &Tags,
    locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
    warnings: &mut RoadWarnings,
) -> ModeResult {
    if tags.is_cycleway(None) {
        if tags.is_cycleway(Some(WaySide::Both))
            || tags.is_cycleway(Some(WaySide::Right))
            || tags.is_cycleway(Some(WaySide::Left))
        {
            return Err(RoadMsg::unsupported_str("cycleway=* with any cycleway:* values").into());
        }
        forward_side.push(Lane::forward(LaneDesignated::Bicycle, locale));
        if oneway {
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
            backward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
        }
    } else if tags.is_cycleway(Some(WaySide::Both)) {
        forward_side.push(Lane::forward(LaneDesignated::Bicycle, locale));
        backward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
    } else {
        // cycleway=opposite_lane
        if tags.is(CYCLEWAY, "opposite_lane") {
            warnings.push(RoadMsg::Deprecated {
                deprecated_tags: tags.subset(&["cycleway", "oneway"]),
                suggested_tags: None,
            });
            backward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
        }
        // cycleway=opposite oneway=yes oneway:bicycle=no
        if tags.is(CYCLEWAY, "opposite") {
            if !(oneway && tags.is("oneway:bicycle", "no")) {
                return Err(RoadMsg::unsupported_str(
                    "cycleway=opposite without oneway=yes oneway:bicycle=no",
                )
                .into());
            }
            backward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
        }
        // cycleway:FORWARD=*
        if tags.is_cycleway(Some(locale.driving_side.into())) {
            if tags.is(CYCLEWAY + locale.driving_side.tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
            {
                forward_side.push(Lane::both(LaneDesignated::Bicycle, locale));
            } else {
                forward_side.push(Lane::forward(LaneDesignated::Bicycle, locale));
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
            forward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
        }
        // cycleway:BACKWARD=*
        if tags.is_cycleway(Some(locale.driving_side.opposite().into())) {
            if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "yes",
            ) {
                forward_side.insert(0, Lane::forward(LaneDesignated::Bicycle, locale));
            } else if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "-1",
            ) {
                backward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
            } else if tags.is(
                CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                "no",
            ) || tags.is("oneway:bicycle", "no")
            {
                backward_side.push(Lane::both(LaneDesignated::Bicycle, locale));
            } else if oneway {
                // A oneway road with a cycleway on the wrong side
                forward_side.insert(0, Lane::forward(LaneDesignated::Bicycle, locale));
            } else {
                // A contraflow bicycle lane
                backward_side.push(Lane::backward(LaneDesignated::Bicycle, locale));
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
