use super::*;

pub fn non_motorized(tags: &Tags, locale: &Locale) -> Result<Option<Lanes>, RoadError> {
    if !tags.is_any(
        HIGHWAY,
        &[
            "cycleway",
            "footway",
            "path",
            "pedestrian",
            "steps",
            "track",
        ],
    ) {
        log::trace!("motorized");
        return Ok(None);
    }
    // Easy special cases first.
    if tags.is(HIGHWAY, "steps") {
        return Ok(Some(Lanes {
            lanes: vec![Lane::foot()],
            warnings: RoadWarnings(vec![RoadMsg::Other {
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
            lanes: vec![Lane::foot()],
            warnings: RoadWarnings::default(),
        }));
    }
    // Otherwise, there'll always be a bike lane.

    let mut forward_side = vec![Lane::forward(LaneDesignated::Bicycle)];
    let mut backward_side = if tags.is("oneway", "yes") {
        vec![]
    } else {
        vec![Lane::backward(LaneDesignated::Bicycle)]
    };

    if !tags.is("foot", "no") {
        forward_side.push(Lane::Shoulder);
        if !backward_side.is_empty() {
            backward_side.push(Lane::Shoulder);
        }
    }
    Ok(Some(Lanes {
        lanes: assemble_ltr(forward_side, backward_side, locale.driving_side)?,
        warnings: RoadWarnings::default(),
    }))
}
