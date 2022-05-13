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
    UnknownVariant(String),
    UnimplementedVariant(String),
}

impl std::fmt::Display for VariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariant(v) => write!(f, "unknown variant '{v}'"),
            Self::UnimplementedVariant(v) => write!(f, "unimplemented variant '{v}'"),
        }
    }
}

impl std::error::Error for VariantError {}

struct Opposite;

fn get_variant<T: AsRef<str>>(
    tags: &Tags,
    k: T,
) -> Result<Option<(Variant, Option<Opposite>)>, VariantError> {
    match tags.get(k) {
        Some("lane") => Ok(Some((Variant::Lane, None))),
        Some("track") => Ok(Some((Variant::Track, None))),
        Some("opposite_lane") => Ok(Some((Variant::Lane, Some(Opposite)))),
        Some("opposite_track") => Ok(Some((Variant::Track, Some(Opposite)))),
        Some("no") | None => Ok(None),
        Some(
            v @ ("opposite"
            | "shared_lane"
            | "share_busway"
            | "opposite_share_busway"
            | "shared"
            | "shoulder"
            | "separate"),
        ) => Err(VariantError::UnimplementedVariant(v.to_owned())),
        Some(v) => Err(VariantError::UnknownVariant(v.to_owned())),
    }
}
fn cycleway_variant(
    tags: &Tags,
    side: Option<WaySide>,
) -> Result<Option<(Variant, Option<Opposite>)>, VariantError> {
    if let Some(side) = side {
        get_variant(tags, CYCLEWAY + side.as_str())
    } else {
        get_variant(tags, CYCLEWAY)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(in crate::transform::tags_to_lanes) enum Variant {
    Lane,
    Track,
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
    #[allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
    pub(in crate::transform::tags_to_lanes) fn from_tags(
        tags: &Tags,
        locale: &Locale,
        road_oneway: Oneway,
        warnings: &mut RoadWarnings,
    ) -> Result<Self, TagsToLanesMsg> {
        if let Ok(Some((variant, opposite))) = cycleway_variant(tags, None) {
            if cycleway_variant(tags, Some(WaySide::Both))
                .ok()
                .flatten()
                .is_some()
                || cycleway_variant(tags, Some(WaySide::Left))
                    .ok()
                    .flatten()
                    .is_some()
                || cycleway_variant(tags, Some(WaySide::Right))
                    .ok()
                    .flatten()
                    .is_some()
            {
                return Err(TagsToLanesMsg::unsupported_str(
                    "cycleway=* with any cycleway:* values",
                ));
            }
            if road_oneway.into() {
                if let Some(Opposite) = opposite {
                    Ok(Self(Location::Forward(Way {
                        variant,
                        direction: Direction::Forward,
                        width: None,
                    })))
                } else {
                    Ok(Self(Location::Backward(Way {
                        variant,
                        direction: Direction::Backward,
                        width: None,
                    })))
                }
            } else {
                if let Some(Opposite) = opposite {
                    warnings.push(TagsToLanesMsg::unsupported_tags(tags.subset(&["cycleway"])));
                }
                Ok(Self(Location::Both {
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
        } else if let Ok(Some((variant, opposite))) = cycleway_variant(tags, Some(WaySide::Both)) {
            if let Some(Opposite) = opposite {
                warnings.push(TagsToLanesMsg::unsupported_tags(
                    tags.subset(&["cycleway:both"]),
                ));
            }
            Ok(Self(Location::Both {
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
        } else {
            // cycleway:FORWARD=*
            if let Ok(Some((variant, _opposite))) =
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
            if let Ok(Some((variant, _opposite))) =
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
    use crate::transform::tags_to_lanes::modes::bicycle::{Location, Variant, Way};
    use crate::transform::tags_to_lanes::oneway::Oneway;
    use crate::transform::RoadWarnings;

    #[test]
    fn cycleway_lane() {
        let scheme = Scheme::from_tags(
            &Tags::from_str_pair(["cycleway", "lane"]),
            &Locale::builder().build(),
            Oneway::No,
            &mut RoadWarnings::default(),
        )
        .unwrap();
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
    #[ignore]
    fn err_cycleway_1() {
        let scheme = Scheme::from_tags(
            &Tags::from_str_pairs(&[["cycleway", "no"], ["cycleway:left", "lane"]]).unwrap(),
            &Locale::builder().build(),
            Oneway::No,
            &mut RoadWarnings::default(),
        );
        assert!(scheme.is_err())
    }

    #[test]
    #[ignore]
    fn err_cycleway_2() {
        let scheme = Scheme::from_tags(
            &Tags::from_str_pairs(&[["cycleway", "track"], ["cycleway:left", "no"]]).unwrap(),
            &Locale::builder().build(),
            Oneway::No,
            &mut RoadWarnings::default(),
        );
        assert!(scheme.is_err())
    }

    #[test]
    #[ignore]
    fn err_cycleway_3() {
        let scheme = Scheme::from_tags(
            &Tags::from_str_pairs(&[["cycleway:both", "lane"], ["cycleway:right", "track"]])
                .unwrap(),
            &Locale::builder().build(),
            Oneway::No,
            &mut RoadWarnings::default(),
        );
        assert!(scheme.is_err())
    }
}
