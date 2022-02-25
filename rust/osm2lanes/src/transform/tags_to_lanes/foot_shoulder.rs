use super::*;

impl LaneBuilder {
    fn shoulder(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Shoulder),
            ..Default::default()
        }
    }
    fn foot(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            designated: Infer::Direct(LaneDesignated::Foot),
            ..Default::default()
        }
    }
    fn is_bicycle(&self) -> bool {
        self.designated.some() == Some(LaneDesignated::Bicycle)
    }
}

pub(super) fn foot_and_shoulder(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
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
    let err = Err(RoadMsg::Unsupported {
        description: None,
        tags: Some(tags.subset(&[
            SIDEWALK,
            SIDEWALK + locale.driving_side.tag(),
            SIDEWALK + locale.driving_side.opposite().tag(),
        ])),
    }
    .into());
    let sidewalk: (Sidewalk, Sidewalk) = match (
        tags.get(SIDEWALK),
        tags.get(SIDEWALK + "both"),
        (
            tags.get(SIDEWALK + locale.driving_side.tag()),
            tags.get(SIDEWALK + locale.driving_side.opposite().tag()),
        ),
    ) {
        // No scheme
        (None, None, (None, None)) => (Sidewalk::None, Sidewalk::None),
        // sidewalk=
        (Some(v), None, (None, None)) => match v {
            "none" => return Err(RoadMsg::deprecated_tag("sidewalk", "none").into()),
            "no" => (Sidewalk::No, Sidewalk::No),
            "yes" => {
                warnings.push(RoadMsg::Ambiguous {
                    description: None,
                    tags: Some(tags.subset(&[SIDEWALK, SIDEWALK + "both"])),
                });
                (Sidewalk::Yes, Sidewalk::Yes)
            }
            "both" => (Sidewalk::Yes, Sidewalk::Yes),
            s if s == locale.driving_side.tag().as_str() => (Sidewalk::Yes, Sidewalk::No),
            s if s == locale.driving_side.opposite().tag().as_str() => {
                (Sidewalk::No, Sidewalk::Yes)
            }
            "separate" => (Sidewalk::Separate, Sidewalk::Separate),
            _ => return err,
        },
        // sidewalk:both=
        (None, Some(v), (None, None)) => match v {
            "no" => (Sidewalk::No, Sidewalk::No),
            "yes" => (Sidewalk::Yes, Sidewalk::Yes),
            "separate" => (Sidewalk::Separate, Sidewalk::Separate),
            _ => return err,
        },
        // sidewalk:left= and/or sidewalk:right=
        (None, None, (forward, backward)) => match (forward, backward) {
            (None, None) => unreachable!(),
            (Some("yes"), Some("yes")) => (Sidewalk::Yes, Sidewalk::Yes),

            (Some("yes"), None | Some("no")) => (Sidewalk::Yes, Sidewalk::No),
            (None | Some("no"), Some("yes")) => (Sidewalk::No, Sidewalk::Yes),

            (Some("separate"), None) => (Sidewalk::Separate, Sidewalk::No),
            (None, Some("separate")) => (Sidewalk::No, Sidewalk::Separate),
            (Some(_), None) | (None, Some(_)) | (Some(_), Some(_)) => {
                return err;
            }
        },
        (Some(_), Some(_), (_, _))
        | (Some(_), _, (_, Some(_)) | (Some(_), _))
        | (_, Some(_), (_, Some(_)) | (Some(_), _)) => {
            return err;
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

    impl RoadBuilder {
        fn lane_outside(&self, forward: bool) -> Option<&LaneBuilder> {
            if forward {
                self.forward_lanes.back()
            } else {
                self.backward_lanes.back()
            }
        }
        fn push_outside(&mut self, lane: LaneBuilder, forward: bool) {
            if forward {
                self.push_forward_outside(lane)
            } else {
                self.push_backward_outside(lane)
            }
        }
        fn add_sidewalk_shoulder(
            &mut self,
            (sidewalk, shoulder): (Sidewalk, Shoulder),
            forward: bool,
            tags: &Tags,
            locale: &Locale,
        ) -> Result<(), RoadError> {
            match (sidewalk, shoulder) {
                (Sidewalk::No | Sidewalk::None, Shoulder::None) => {
                    // We assume a shoulder if there is no bike lane.
                    // This assumes bicycle lanes are just glorified shoulders...
                    let has_bicycle_lane = self
                        .lane_outside(forward)
                        .map_or(false, |lane| lane.is_bicycle());
                    if !has_bicycle_lane && (forward || !bool::from(self.oneway)) {
                        self.push_outside(LaneBuilder::shoulder(locale), forward)
                    }
                }
                (Sidewalk::No | Sidewalk::None, Shoulder::No) => {}
                (Sidewalk::Yes, Shoulder::No | Shoulder::None) => {
                    self.push_outside(LaneBuilder::foot(locale), forward)
                }
                (Sidewalk::No | Sidewalk::None, Shoulder::Yes) => {
                    self.push_outside(LaneBuilder::shoulder(locale), forward)
                }
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
        }
    }

    road.add_sidewalk_shoulder((sidewalk.0, shoulder.0), true, tags, locale)?;
    road.add_sidewalk_shoulder((sidewalk.1, shoulder.1), false, tags, locale)?;

    Ok(())
}
