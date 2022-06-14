use celes::Country;
use osm_tags::Tags;

use crate::locale::Locale;
use crate::metric::Metre;
use crate::road::Designated;
use crate::transform::tags::{SHOULDER, SIDEWALK};
use crate::transform::tags_to_lanes::road::Width;
use crate::transform::tags_to_lanes::{Infer, LaneBuilder, LaneType, RoadBuilder, TagsToLanesMsg};
use crate::transform::{RoadError, RoadWarnings};

impl LaneBuilder {
    fn shoulder(locale: &Locale) -> Self {
        let mut shoulder = Self {
            r#type: Infer::Direct(LaneType::Shoulder),
            ..Default::default()
        };
        if let Some(c) = &locale.country {
            if c == &Country::the_netherlands() {
                shoulder.width = Width {
                    target: Infer::Default(Metre::new(0.6)),
                    ..Default::default()
                }
            }
        }
        shoulder
    }
    fn foot(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            designated: Infer::Direct(Designated::Foot),
            ..Default::default()
        }
    }
    fn is_bicycle(&self) -> bool {
        self.designated.some() == Some(Designated::Bicycle)
    }
}

enum Sidewalk {
    None,
    No,
    Yes,
    Separate,
}

impl Sidewalk {
    /// This processes sidewalk tags by the OSM spec.
    /// No can be implied, e.g. we assume that sidewalk:left=yes implies sidewalk:right=no
    /// None is when information may be incomplete and should be inferred,
    /// e.g. when sidewalk=* is missing altogether,
    /// but this may later become a No when combined with data from shoulder=*
    /// We catch any tag combinations that violate the OSM spec
    #[allow(clippy::unnested_or_patterns)]
    fn from_tags(
        tags: &Tags,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Result<(Self, Self), TagsToLanesMsg> {
        let err = Err(TagsToLanesMsg::unsupported_tags(tags.subset(&[
            SIDEWALK,
            SIDEWALK + locale.driving_side.tag(),
            SIDEWALK + locale.driving_side.opposite().tag(),
        ])));
        let sidewalks = match (
            tags.get(&SIDEWALK),
            tags.get(&(SIDEWALK + "both")),
            (
                tags.get(&(SIDEWALK + locale.driving_side.tag())),
                tags.get(&(SIDEWALK + locale.driving_side.opposite().tag())),
            ),
        ) {
            (Some(v), None, (None, None)) => match v {
                "none" => return Err(TagsToLanesMsg::deprecated_tag("sidewalk", "none")),
                "no" => (Sidewalk::No, Sidewalk::No),
                "yes" => {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(
                        tags.subset(&[SIDEWALK, SIDEWALK + "both"]),
                    ));
                    (Sidewalk::Yes, Sidewalk::Yes)
                },
                "both" => (Sidewalk::Yes, Sidewalk::Yes),
                s if s == locale.driving_side.tag().as_str() => (Sidewalk::Yes, Sidewalk::No),
                s if s == locale.driving_side.opposite().tag().as_str() => {
                    (Sidewalk::No, Sidewalk::Yes)
                },
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
                // no scheme
                (None, None) => (Sidewalk::None, Sidewalk::None),

                (Some("yes"), Some("yes")) => (Sidewalk::Yes, Sidewalk::Yes),

                (Some("yes"), None | Some("no")) => (Sidewalk::Yes, Sidewalk::No),
                (None | Some("no"), Some("yes")) => (Sidewalk::No, Sidewalk::Yes),

                (Some("separate"), None) => (Sidewalk::Separate, Sidewalk::No),
                (None, Some("separate")) => (Sidewalk::No, Sidewalk::Separate),
                (Some(_), None) | (None, Some(_)) | (Some(_), Some(_)) => {
                    return err;
                },
            },
            (Some(_), Some(_), (_, _))
            | (Some(_), _, (_, Some(_)) | (Some(_), _))
            | (_, Some(_), (_, Some(_)) | (Some(_), _)) => {
                return err;
            },
        };
        Ok(sidewalks)
    }
}

enum Shoulder {
    None,
    Yes,
    No,
}

impl Shoulder {
    fn from_tags(
        tags: &Tags,
        locale: &Locale,
        _warnings: &mut RoadWarnings,
    ) -> Result<(Self, Self), TagsToLanesMsg> {
        Ok(match tags.get(&SHOULDER) {
            None => (Shoulder::None, Shoulder::None),
            Some("no") => (Shoulder::No, Shoulder::No),
            Some("yes" | "both") => (Shoulder::Yes, Shoulder::Yes),
            Some(s) if s == locale.driving_side.tag().as_str() => (Shoulder::Yes, Shoulder::No),
            Some(s) if s == locale.driving_side.opposite().tag().as_str() => {
                (Shoulder::No, Shoulder::Yes)
            },
            Some(s) => return Err(TagsToLanesMsg::unsupported_tag(SHOULDER, s)),
        })
    }
}

#[allow(clippy::items_after_statements, clippy::unnested_or_patterns)]
pub(in crate::transform::tags_to_lanes) fn foot_and_shoulder(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), RoadError> {
    // https://wiki.openstreetmap.org/wiki/Key:sidewalk
    let sidewalk: (Sidewalk, Sidewalk) = Sidewalk::from_tags(tags, locale, warnings)?;

    // https://wiki.openstreetmap.org/wiki/Key:shoulder
    let shoulder: (Shoulder, Shoulder) = Shoulder::from_tags(tags, locale, warnings)?;

    impl RoadBuilder {
        fn lane_outside(&self, forward: bool) -> Option<&LaneBuilder> {
            if forward {
                self.forward_outside()
            } else {
                self.backward_outside()
            }
        }
        fn push_outside(&mut self, lane: LaneBuilder, forward: bool) {
            if forward {
                self.push_forward_outside(lane);
            } else {
                self.push_backward_outside(lane);
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
                        .map_or(false, LaneBuilder::is_bicycle);
                    if !has_bicycle_lane
                        && locale.has_shoulder(self.highway.r#type())
                        && (forward || !bool::from(self.oneway))
                        && !tags.is("parking:condition:both", "no_stopping")
                    {
                        self.push_outside(LaneBuilder::shoulder(locale), forward);
                    }
                },
                (Sidewalk::No | Sidewalk::None, Shoulder::No) => {},
                (Sidewalk::Yes, Shoulder::No | Shoulder::None) => {
                    self.push_outside(LaneBuilder::foot(locale), forward);
                },
                (Sidewalk::No | Sidewalk::None, Shoulder::Yes) => {
                    self.push_outside(LaneBuilder::shoulder(locale), forward);
                },
                (Sidewalk::Yes, Shoulder::Yes) => {
                    return Err(TagsToLanesMsg::unsupported(
                        "shoulder and sidewalk on same side",
                        tags.subset(&[SIDEWALK, SHOULDER]),
                    )
                    .into());
                },
                (Sidewalk::Separate, _) => {
                    return Err(TagsToLanesMsg::unsupported_tag(SIDEWALK, "separate").into())
                },
            }
            Ok(())
        }
    }

    road.add_sidewalk_shoulder((sidewalk.0, shoulder.0), true, tags, locale)?;
    road.add_sidewalk_shoulder((sidewalk.1, shoulder.1), false, tags, locale)?;

    Ok(())
}
