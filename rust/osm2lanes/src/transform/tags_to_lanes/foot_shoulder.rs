use super::*;

pub fn foot_and_shoulder(
    tags: &Tags,
    locale: &Locale,
    oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
    warnings: &mut RoadWarnings,
) -> ModeResult {
    // https://wiki.openstreetmap.org/wiki/Key:sidewalk
    // This first step processes tags by the OSM spec.
    // No can be implied, e.g. we assume that sidewalk:left=yes implies sidewalk:right=no
    // None is when information may be incomplete and should be inferred,
    // e.g. when sidewalk=* is missing altogether,
    // but this may later become a No when combined with data from shoulder=*
    // We catch any tag combinations that violate the OSM spec
    enum Sidewalk {
        None,
        No,
        Yes,
        Separate,
    }
    let sidewalk: (Sidewalk, Sidewalk) = match (
        tags.get(SIDEWALK),
        tags.get(SIDEWALK + locale.driving_side.tag()),
        tags.get(SIDEWALK + locale.driving_side.opposite().tag()),
    ) {
        (None, None, None) => (Sidewalk::None, Sidewalk::None),
        (Some("none"), None, None) => {
            return Err(RoadMsg::deprecated_tag("sidewalk", "none").into())
        }
        (Some("no"), None, None) => (Sidewalk::No, Sidewalk::No),
        (Some("yes"), None, None) => {
            warnings.push(RoadMsg::Ambiguous {
                description: None,
                tags: Some(tags.subset(&[SIDEWALK])),
            });
            (Sidewalk::Yes, Sidewalk::Yes)
        }
        (Some("both"), None, None) => (Sidewalk::Yes, Sidewalk::Yes),
        (None, Some("yes"), Some("yes")) => (Sidewalk::Yes, Sidewalk::Yes),
        (Some(s), None, None) if s == locale.driving_side.tag().as_str() => {
            (Sidewalk::Yes, Sidewalk::No)
        }
        (None, Some("yes"), None | Some("no")) => (Sidewalk::Yes, Sidewalk::No),
        (Some(s), None, None) if s == locale.driving_side.opposite().tag().as_str() => {
            (Sidewalk::No, Sidewalk::Yes)
        }
        (None, None | Some("no"), Some("yes")) => (Sidewalk::No, Sidewalk::Yes),
        (Some("separate"), None, None) => (Sidewalk::Separate, Sidewalk::Separate),
        (None, Some("separate"), None) => (Sidewalk::Separate, Sidewalk::No),
        (None, None, Some("separate")) => (Sidewalk::No, Sidewalk::Separate),
        // TODO: generate the rest of these automatically
        (None, Some(forward), None) => {
            return Err(
                RoadMsg::unsupported_tag(SIDEWALK + locale.driving_side.tag(), forward).into(),
            )
        }
        (None, None, Some(backward)) => {
            return Err(RoadMsg::unsupported_tag(
                SIDEWALK + locale.driving_side.opposite().tag(),
                backward,
            )
            .into())
        }
        (Some(_s), Some(_forward), None) => {
            return Err(RoadMsg::Unsupported {
                description: None,
                tags: Some(tags.subset(&[SIDEWALK, SIDEWALK + locale.driving_side.tag()])),
            }
            .into())
        }
        (Some(_s), None, Some(_backward)) => {
            return Err(RoadMsg::Unsupported {
                description: None,
                tags: Some(
                    tags.subset(&[SIDEWALK, SIDEWALK + locale.driving_side.opposite().tag()]),
                ),
            }
            .into())
        }
        (Some(s), None, None) => return Err(RoadMsg::unsupported_tag(SIDEWALK, s).into()),
        (None, Some(_forward), Some(_backward)) => {
            return Err(RoadMsg::Unsupported {
                description: None,
                tags: Some(tags.subset(&[
                    SIDEWALK + locale.driving_side.tag(),
                    SIDEWALK + locale.driving_side.opposite().tag(),
                ])),
            }
            .into())
        }
        (Some(_s), Some(_forward), Some(_backward)) => {
            return Err(RoadMsg::Unsupported {
                description: None,
                tags: Some(tags.subset(&[
                    SIDEWALK,
                    SIDEWALK + locale.driving_side.tag(),
                    SIDEWALK + locale.driving_side.opposite().tag(),
                ])),
            }
            .into())
        }
    };

    // https://wiki.openstreetmap.org/wiki/Key:shoulder
    enum Shoulder {
        None,
        Yes,
        No,
    }
    let shoulder: (Shoulder, Shoulder) = match tags.get(SHOULDER) {
        None => (Shoulder::None, Shoulder::None),
        Some("no") => (Shoulder::No, Shoulder::No),
        Some("yes") => (Shoulder::Yes, Shoulder::Yes),
        Some("both") => (Shoulder::Yes, Shoulder::Yes),
        Some(s) if s == locale.driving_side.tag().as_str() => (Shoulder::Yes, Shoulder::No),
        Some(s) if s == locale.driving_side.opposite().tag().as_str() => {
            (Shoulder::No, Shoulder::Yes)
        }
        Some(s) => return Err(RoadMsg::unsupported_tag(SHOULDER, s).into()),
    };

    let add = |(sidewalk, shoulder): (Sidewalk, Shoulder),
               side: &mut Vec<Lane>,
               forward: bool|
     -> ModeResult {
        match (sidewalk, shoulder) {
            (Sidewalk::No | Sidewalk::None, Shoulder::None) => {
                // We assume a shoulder if there is no bike lane.
                // This assumes bicycle lanes are just glorified shoulders...
                let has_bicycle_lane = side.last().map_or(false, |lane| lane.is_bicycle());

                if !has_bicycle_lane && (forward || !oneway) {
                    side.push(Lane::Shoulder)
                }
            }
            (Sidewalk::No | Sidewalk::None, Shoulder::No) => {}
            (Sidewalk::Yes, Shoulder::No | Shoulder::None) => side.push(Lane::foot()),
            (Sidewalk::No | Sidewalk::None, Shoulder::Yes) => side.push(Lane::Shoulder),
            (Sidewalk::Yes, Shoulder::Yes) => {
                return Err(RoadMsg::Unsupported {
                    description: Some("shoulder and sidewalk on same side".to_owned()),
                    tags: Some(tags.subset(&[SIDEWALK, SHOULDER])),
                }
                .into());
            }
            (Sidewalk::Separate, _) => {
                return Err(RoadMsg::unsupported_tag(SIDEWALK, "separate").into())
            }
        }
        Ok(())
    };

    add((sidewalk.0, shoulder.0), forward_side, true)?;
    add((sidewalk.1, shoulder.1), backward_side, false)?;

    Ok(())
}
