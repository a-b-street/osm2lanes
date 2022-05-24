use std::fmt::Display;

use crate::locale::Locale;
use crate::metric::Metre;
use crate::road::{Designated, Direction};
use crate::tag::Tags;
use crate::transform::tags::CYCLEWAY;
use crate::transform::tags_to_lanes::oneway::Oneway;
use crate::transform::tags_to_lanes::road::{LaneType, Width};
use crate::transform::tags_to_lanes::{
    Infer, LaneBuilder, RoadBuilder, TagsNumeric, TagsToLanesMsg,
};
use crate::transform::{RoadWarnings, WaySide};

#[derive(Debug)]
enum VariantError {
    UnknownVariant(String, String),
    UnimplementedVariant(String, String),
}

impl From<VariantError> for TagsToLanesMsg {
    fn from(e: VariantError) -> Self {
        match e {
            VariantError::UnknownVariant(key, val) => Self::unsupported_tag(key, &val),
            VariantError::UnimplementedVariant(key, val) => Self::unimplemented_tag(key, &val),
        }
    }
}

struct Opposite;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(in crate::transform::tags_to_lanes) enum Variant {
    SharedMotor,
    // SharedBus,
    // OptionalLane,
    Lane,
    Track,
}

impl Display for Variant {
    #[allow(clippy::todo, clippy::panic_in_result_fn)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::SharedMotor => todo!(),
                Self::Lane => "lane",
                Self::Track => "track",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::transform::tags_to_lanes) enum OptionNo<T> {
    None,
    No,
    Some(T),
}

impl<T> OptionNo<T> {
    fn _some(self) -> Option<T> {
        match self {
            OptionNo::None | OptionNo::No => None,
            OptionNo::Some(v) => Some(v),
        }
    }
}

fn get_variant<T: AsRef<str>>(
    tags: &Tags,
    k: T,
) -> Result<OptionNo<(Variant, Option<Opposite>)>, VariantError> {
    match tags.get(&k) {
        Some("lane") => Ok(OptionNo::Some((Variant::Lane, None))),
        Some("track") => Ok(OptionNo::Some((Variant::Track, None))),
        Some("opposite_lane") => Ok(OptionNo::Some((Variant::Lane, Some(Opposite)))),
        Some("opposite_track") => Ok(OptionNo::Some((Variant::Track, Some(Opposite)))),
        Some("opposite") => Ok(OptionNo::Some((Variant::SharedMotor, Some(Opposite)))),
        Some("no") => Ok(OptionNo::No),
        Some(
            v @ ("shared_lane"
            | "share_busway"
            | "opposite_share_busway"
            | "shared"
            | "shoulder"
            | "separate"),
        ) => Err(VariantError::UnimplementedVariant(
            k.as_ref().to_owned(),
            v.to_owned(),
        )),
        Some(v) => Err(VariantError::UnknownVariant(
            k.as_ref().to_owned(),
            v.to_owned(),
        )),
        None => Ok(OptionNo::None),
    }
}

fn cycleway_variant(
    tags: &Tags,
    side: Option<WaySide>,
) -> Result<OptionNo<(Variant, Option<Opposite>)>, VariantError> {
    if let Some(side) = side {
        get_variant(tags, CYCLEWAY + side.as_str())
    } else {
        get_variant(tags, CYCLEWAY)
    }
}

#[derive(Debug, PartialEq)]
pub(in crate::transform::tags_to_lanes) struct Way {
    variant: Variant,
    direction: Direction,
    width: Option<Width>,
}

#[derive(Debug, PartialEq)]
pub(in crate::transform::tags_to_lanes) enum Location {
    None,
    Forward(Way),
    Backward(Way),
    Both { forward: Way, backward: Way },
}

/// Bicycle lane or track scheme
#[derive(Debug, PartialEq)]
pub(in crate::transform::tags_to_lanes) struct Scheme(Location);

impl Scheme {
    #[allow(
        clippy::unnecessary_wraps,
        clippy::too_many_lines,
        clippy::panic_in_result_fn
    )]
    pub(in crate::transform::tags_to_lanes) fn from_tags(
        tags: &Tags,
        locale: &Locale,
        road_oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        let scheme_cycleway = Self::from_tags_cycleway(tags, locale, road_oneway, warnings);
        let scheme_cycleway_both =
            Self::from_tags_cycleway_both(tags, locale, road_oneway, warnings);

        if let Some(scheme_cycleway) = scheme_cycleway {
            return Ok(scheme_cycleway);
        }

        if let Some(scheme_cycleway_both) = scheme_cycleway_both {
            return Ok(scheme_cycleway_both);
        }

        // cycleway:FORWARD=*
        if let Ok(OptionNo::Some((variant, _opposite))) =
            cycleway_variant(tags, Some(locale.driving_side.into()))
        {
            let width = tags
                .get_parsed(CYCLEWAY + locale.driving_side.tag() + "width", warnings)
                .map(|w| Width {
                    target: Infer::Direct(Metre::new(w)),
                    ..Default::default()
                });
            if tags.is(CYCLEWAY + locale.driving_side.tag() + "oneway", "no")
                || tags.is("oneway:bicycle", "no")
            {
                return Ok(Self(Location::Forward(Way {
                    variant,
                    direction: Direction::Both,
                    width,
                })));
            }
            return Ok(Self(Location::Forward(Way {
                variant,
                direction: Direction::Forward,
                width,
            })));
        }
        // cycleway:FORWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            warnings.push(TagsToLanesMsg::deprecated_tags(
                tags.subset(&[CYCLEWAY + locale.driving_side.tag()]),
            ));
            return Ok(Self(Location::Forward(Way {
                variant: Variant::Lane, // TODO distinguish oposite_ values
                direction: Direction::Backward,
                width: None,
            })));
        }
        // cycleway:BACKWARD=*
        if let Ok(OptionNo::Some((variant, _opposite))) =
            cycleway_variant(tags, Some(locale.driving_side.opposite().into()))
        {
            let width = tags
                .get_parsed(
                    CYCLEWAY + locale.driving_side.opposite().tag() + "width",
                    warnings,
                )
                .map(|w| Width {
                    target: Infer::Direct(Metre::new(w)),
                    ..Default::default()
                });
            return Ok(Self(
                if tags.is(
                    CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                    "yes",
                ) {
                    Location::Backward(Way {
                        variant,
                        direction: Direction::Forward,
                        width,
                    })
                } else if tags.is(
                    CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                    "-1",
                ) {
                    Location::Backward(Way {
                        variant,
                        direction: Direction::Backward,
                        width,
                    })
                } else if tags.is(
                    CYCLEWAY + locale.driving_side.opposite().tag() + "oneway",
                    "no",
                ) || tags.is("oneway:bicycle", "no")
                {
                    Location::Backward(Way {
                        variant,
                        direction: Direction::Both,
                        width,
                    })
                } else if road_oneway.into() {
                    // A oneway road with a cycleway on the wrong side
                    Location::Backward(Way {
                        variant,
                        direction: Direction::Forward,
                        width,
                    })
                } else {
                    // A contraflow bicycle lane
                    Location::Backward(Way {
                        variant,
                        direction: Direction::Backward,
                        width,
                    })
                },
            ));
        }
        // cycleway:BACKWARD=opposite_lane
        if tags.is_any(
            CYCLEWAY + locale.driving_side.opposite().tag(),
            &["opposite_lane", "opposite_track"],
        ) {
            return Err(TagsToLanesMsg::unsupported_tags(
                tags.subset(&[CYCLEWAY + locale.driving_side.opposite().tag()]),
            ));
        }
        Ok(Self(Location::None))
    }

    /// Handle `cycleway=*` tags
    #[allow(clippy::unnecessary_wraps, clippy::panic_in_result_fn)]
    pub(in crate::transform::tags_to_lanes) fn from_tags_cycleway(
        tags: &Tags,
        locale: &Locale,
        road_oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Option<Self> {
        match cycleway_variant(tags, None) {
            Ok(OptionNo::Some((variant, opposite))) => {
                if road_oneway.into() {
                    if opposite.is_none() {
                        Some(Self(Location::Forward(Way {
                            variant,
                            direction: Direction::Forward,
                            width: None,
                        })))
                    } else {
                        if let Variant::Lane | Variant::Track = variant {
                            warnings.push(TagsToLanesMsg::deprecated(
                                tags.subset(&["cyleway"]),
                                Tags::from_str_pairs(&[
                                    [
                                        (CYCLEWAY + locale.driving_side.opposite().tag()).as_str(),
                                        &variant.to_string(),
                                    ],
                                    [
                                        (CYCLEWAY
                                            + locale.driving_side.opposite().tag()
                                            + "oneway")
                                            .as_str(),
                                        "-1",
                                    ],
                                ])
                                .unwrap(),
                            ));
                        }
                        Some(Self(Location::Backward(Way {
                            variant,
                            direction: Direction::Backward,
                            width: None,
                        })))
                    }
                } else {
                    if opposite.is_some() {
                        warnings.push(TagsToLanesMsg::unsupported_tags(
                            tags.subset(&["oneway", "cycleway"]),
                        ));
                    }
                    Some(Self(Location::Both {
                        forward: Way {
                            variant,
                            direction: Direction::Forward,
                            width: None,
                        },
                        backward: Way {
                            variant,
                            direction: Direction::Backward,
                            width: None,
                        },
                    }))
                }
            },
            Ok(OptionNo::No) => Some(Self(Location::None)),
            Ok(OptionNo::None) => None,
            Err(e) => {
                warnings.push(e.into());
                None
            },
        }
    }

    /// Handle `cycleway=*` tags
    #[allow(clippy::unnecessary_wraps, clippy::panic_in_result_fn)]
    pub(in crate::transform::tags_to_lanes) fn from_tags_cycleway_both(
        tags: &Tags,
        _locale: &Locale,
        _road_oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Option<Self> {
        match cycleway_variant(tags, Some(WaySide::Both)) {
            Ok(OptionNo::Some((variant, opposite))) => {
                if let Some(Opposite) = opposite {
                    warnings.push(TagsToLanesMsg::unsupported_tags(
                        tags.subset(&["cycleway:both"]),
                    ));
                }
                Some(Self(Location::Both {
                    forward: Way {
                        variant,
                        direction: Direction::Forward,
                        width: None,
                    },
                    backward: Way {
                        variant,
                        direction: Direction::Backward,
                        width: None,
                    },
                }))
            },
            Ok(OptionNo::No) => Some(Self(Location::None)),
            Ok(OptionNo::None) => None,
            Err(e) => {
                warnings.push(e.into());
                None
            },
        }
    }
}

impl LaneBuilder {
    fn cycle_forward(width: Option<Width>, _locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(Direction::Forward),
            designated: Infer::Direct(Designated::Bicycle),
            width: width.unwrap_or_default(),
            ..Default::default()
        }
    }
    fn cycle_backward(width: Option<Width>, _locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(Direction::Backward),
            designated: Infer::Direct(Designated::Bicycle),
            width: width.unwrap_or_default(),
            ..Default::default()
        }
    }
    fn cycle_both(width: Option<Width>, _locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Travel),
            direction: Infer::Direct(Direction::Both),
            designated: Infer::Direct(Designated::Bicycle),
            width: width.unwrap_or_default(),
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
    let scheme = Scheme::from_tags(tags, locale, road.oneway, warnings)?;
    log::trace!("cycleway scheme: {scheme:?}");
    let lane = |way: Way| match way.direction {
        Direction::Forward => LaneBuilder::cycle_forward(way.width, locale),
        Direction::Backward => LaneBuilder::cycle_backward(way.width, locale),
        Direction::Both => LaneBuilder::cycle_both(way.width, locale),
    };
    match scheme.0 {
        Location::None => {},
        Location::Forward(way) => {
            road.push_forward_outside(lane(way));
        },
        Location::Backward(way) => {
            road.push_backward_outside(lane(way));
        },
        Location::Both { forward, backward } => {
            road.push_forward_outside(lane(forward));
            road.push_backward_outside(lane(backward));
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Scheme;
    use crate::locale::Locale;
    use crate::road::Direction;
    use crate::tag::Tags;
    use crate::transform::tags_to_lanes::error::TagsToLanesIssue;
    use crate::transform::tags_to_lanes::modes::bicycle::{Location, Variant, Way};
    use crate::transform::tags_to_lanes::oneway::Oneway;
    use crate::transform::RoadWarnings;

    #[test]
    fn lane() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway", "lane"]),
            &Locale::builder().build(),
            Oneway::No,
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{:?}", warnings);
        assert_eq!(
            scheme,
            Scheme(Location::Both {
                forward: Way {
                    variant: Variant::Lane,
                    direction: Direction::Forward,
                    width: None,
                },
                backward: Way {
                    variant: Variant::Lane,
                    direction: Direction::Backward,
                    width: None,
                }
            })
        )
    }

    #[test]
    fn oneway_opposite_track() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway", "opposite_track"]),
            &Locale::builder().build(),
            Oneway::Yes,
            &mut warnings,
        )
        .unwrap();
        // TODO: expect deprecation warning
        assert_eq!(
            scheme,
            Scheme(Location::Backward(Way {
                variant: Variant::Track,
                direction: Direction::Backward,
                width: None,
            }))
        );
    }

    #[test]
    fn forward_lane() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway:right", "lane"]),
            &Locale::builder().build(),
            Oneway::No,
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{:?}", warnings);
        assert_eq!(
            scheme,
            Scheme(Location::Forward(Way {
                variant: Variant::Lane,
                direction: Direction::Forward,
                width: None,
            }))
        );
    }

    #[test]
    fn backward_track() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway:left", "track"]),
            &Locale::builder().build(),
            Oneway::No,
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{:?}", warnings);
        assert_eq!(
            scheme,
            Scheme(Location::Backward(Way {
                variant: Variant::Track,
                direction: Direction::Backward,
                width: None,
            }))
        );
    }

    #[test]
    fn backward_opposite_track() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway:left", "opposite_track"]),
            &Locale::builder().build(),
            Oneway::No,
            &mut warnings,
        )
        .unwrap();
        // TODO: assert expecting a deprecation warning
        assert_eq!(
            scheme,
            Scheme(Location::Backward(Way {
                variant: Variant::Track,
                direction: Direction::Backward,
                width: None,
            }))
        );
    }

    #[test]
    fn backward_lane_min1() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pairs(&[["cycleway:left", "track"], ["cycleway:left:oneway", "-1"]])
                .unwrap(),
            &Locale::builder().build(),
            Oneway::No,
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{:?}", warnings);
        assert_eq!(
            scheme,
            Scheme(Location::Backward(Way {
                variant: Variant::Track,
                direction: Direction::Backward,
                width: None,
            }))
        );
    }

    #[test]
    fn opposite() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway", "opposite"]),
            &Locale::builder().build(),
            Oneway::Yes,
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{:?}", warnings);
        assert_eq!(
            scheme,
            Scheme(Location::Backward(Way {
                variant: Variant::SharedMotor,
                direction: Direction::Backward,
                width: None,
            }))
        );
    }

    #[test]
    fn warn_shoulder() {
        let mut warnings = RoadWarnings::default();
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway", "shoulder"]),
            &Locale::builder().build(),
            Oneway::No,
            &mut warnings,
        );
        assert!(!warnings.is_empty(), "{:?}", scheme);
    }

    #[test]
    #[ignore]
    fn warn_no_lane() {
        let tags = Tags::from_str_pairs(&[["cycleway", "no"], ["cycleway:left", "lane"]]).unwrap();
        let mut warnings = RoadWarnings::default();
        let _scheme =
            Scheme::from_tags(&tags, &Locale::builder().build(), Oneway::No, &mut warnings);
        assert_eq!(warnings.as_slice().len(), 1);
        assert!(matches!(
            &warnings.as_slice().get(0).unwrap().issue,
            TagsToLanesIssue::Deprecated {
                deprecated_tags,
                suggested_tags: None,
            } if deprecated_tags.to_str_pairs() == tags.to_str_pairs()
        ));
    }

    #[test]
    #[ignore]
    fn err_track_no() {
        let scheme = Scheme::from_tags(
            &Tags::from_str_pairs(&[["cycleway", "track"], ["cycleway:left", "no"]]).unwrap(),
            &Locale::builder().build(),
            Oneway::No,
            &mut RoadWarnings::default(),
        );
        assert!(scheme.is_err());
    }

    #[test]
    #[ignore]
    fn err_lane_track() {
        let scheme = Scheme::from_tags(
            &Tags::from_str_pairs(&[["cycleway:both", "lane"], ["cycleway:right", "track"]])
                .unwrap(),
            &Locale::builder().build(),
            Oneway::No,
            &mut RoadWarnings::default(),
        );
        assert!(scheme.is_err());
    }
}
