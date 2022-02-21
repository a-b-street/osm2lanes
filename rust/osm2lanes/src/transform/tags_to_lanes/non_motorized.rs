use super::*;

impl Lane {
    fn shoulder(locale: &Locale) -> Self {
        Self::Shoulder {
            // TODO: width not just motor
            width: Some(locale.travel_width(&LaneDesignated::Motor)),
        }
    }
    fn foot(locale: &Locale) -> Self {
        let designated = LaneDesignated::Foot;
        Self::Travel {
            direction: None,
            designated,
            width: Some(locale.travel_width(&designated)),
        }
    }
}

pub fn non_motorized(
    tags: &Tags,
    locale: &Locale,
    highway: Highway,
) -> Result<Option<Lanes>, RoadError> {
    if let Highway::Cycleway
    | Highway::Footway
    | Highway::Path
    | Highway::Pedestrian
    | Highway::Steps
    | Highway::Track = highway
    {
        log::trace!("non-motorized");
    } else {
        log::trace!("motorized");
        return Ok(None);
    }
    // Easy special cases first.
    if tags.is(HIGHWAY, "steps") {
        return Ok(Some(Lanes {
            lanes: vec![Lane::foot(locale)],
            warnings: RoadWarnings::new(vec![RoadMsg::Other {
                description: "highway is steps, but lane is only a sidewalk".to_owned(),
                tags: tags.subset(&[HIGHWAY]),
            }]),
        }));
    }

    // Eventually, we should have some kind of special LaneType for shared walking/cycling paths of
    // different kinds. Until then, model by making bike lanes and a shoulder for walking.

    // If it just allows foot traffic, simply make it a sidewalk. For most of the above highway
    // types, assume bikes are allowed, except for footways, where they must be explicitly
    // allowed.
    if tags.is("bicycle", "no")
        || (tags.is(HIGHWAY, "footway") && !tags.is_any("bicycle", &["designated", "yes"]))
    {
        return Ok(Some(Lanes {
            lanes: vec![Lane::foot(locale)],
            warnings: RoadWarnings::default(),
        }));
    }
    // Otherwise, there'll always be a bike lane.

    let mut forward_side = vec![Lane::forward(LaneDesignated::Bicycle, locale)];
    let mut backward_side = if tags.is("oneway", "yes") {
        vec![]
    } else {
        vec![Lane::backward(LaneDesignated::Bicycle, locale)]
    };

    if !tags.is("foot", "no") {
        forward_side.push(Lane::shoulder(locale));
        if !backward_side.is_empty() {
            backward_side.push(Lane::shoulder(locale));
        }
    }
    Ok(Some(Lanes {
        lanes: assemble_ltr(forward_side, backward_side, locale.driving_side)?,
        warnings: RoadWarnings::default(),
    }))
}
