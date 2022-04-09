use crate::locale::{DrivingSide, Locale};
use crate::road::{Designated, Lane, Road};
use crate::tag::{Tags, HIGHWAY};
use crate::transform::tags_to_lanes::RoadBuilder;
use crate::transform::{RoadError, RoadFromTags, RoadMsg, RoadWarnings};

impl Lane {
    fn shoulder(locale: &Locale) -> Self {
        Self::Shoulder {
            // TODO: width not just motor
            width: Some(locale.travel_width(&Designated::Motor)),
        }
    }
    fn foot(locale: &Locale) -> Self {
        let designated = Designated::Foot;
        Self::Travel {
            direction: None,
            designated,
            width: Some(locale.travel_width(&designated)),
            max_speed: None,
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(in crate::transform::tags_to_lanes) fn non_motorized(
    tags: &Tags,
    locale: &Locale,
    road: &RoadBuilder,
) -> Result<Option<RoadFromTags>, RoadError> {
    if road.highway.is_supported_non_motorized() {
        log::trace!("non-motorized");
    } else {
        log::trace!("motorized");
        return Ok(None);
    }
    // Easy special cases first.
    if tags.is(HIGHWAY, "steps") || tags.is(HIGHWAY, "path") {
        return Ok(Some(RoadFromTags {
            road: Road {
                lanes: vec![Lane::foot(locale)],
                highway: road.highway.clone(),
            },
            warnings: RoadWarnings::new(if tags.is(HIGHWAY, "steps") {
                vec![RoadMsg::Other {
                    description: "highway is steps, but lane is only a sidewalk".to_owned(),
                    tags: tags.subset(&[HIGHWAY]),
                }]
            } else {
                Vec::new()
            }),
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
        return Ok(Some(RoadFromTags {
            road: Road {
                lanes: vec![Lane::foot(locale)],
                highway: road.highway.clone(),
            },
            warnings: RoadWarnings::default(),
        }));
    }
    // Otherwise, there'll always be a bike lane.

    let mut forward_side = vec![Lane::forward(Designated::Bicycle, locale)];
    let mut backward_side = if tags.is("oneway", "yes") {
        vec![]
    } else {
        vec![Lane::backward(Designated::Bicycle, locale)]
    };

    if !tags.is("foot", "no") {
        forward_side.push(Lane::shoulder(locale));
        if !backward_side.is_empty() {
            backward_side.push(Lane::shoulder(locale));
        }
    }
    Ok(Some(RoadFromTags {
        road: Road {
            lanes: assemble_ltr(forward_side, backward_side, locale.driving_side),
            highway: road.highway.clone(),
        },
        warnings: RoadWarnings::default(),
    }))
}

fn assemble_ltr(
    mut fwd_side: Vec<Lane>,
    mut back_side: Vec<Lane>,
    driving_side: DrivingSide,
) -> Vec<Lane> {
    match driving_side {
        DrivingSide::Right => {
            back_side.reverse();
            back_side.extend(fwd_side);
            back_side
        },
        DrivingSide::Left => {
            fwd_side.reverse();
            fwd_side.extend(back_side);
            fwd_side
        },
    }
}
