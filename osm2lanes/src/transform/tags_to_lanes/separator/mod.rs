use celes::Country;

use crate::locale::Locale;
use crate::metric::Metre;
use crate::road::{Color, Designated, Direction, Lane, Marking, Markings, Style};
use crate::tag::Tags;
use crate::transform::{RoadWarnings, TagsToLanesMsg};

mod semantic;

use semantic::{Overtake, Separator, SpeedClass};

use self::semantic::{EdgeSeparator, LaneChange, ParkingCondition};
use super::{LaneBuilder, LaneType, RoadBuilder};

#[derive(Clone, Copy)]
enum DirectionChange {
    // One of the sides is bidirectional
    None,
    Same,
    Opposite,
}

/// Given a pair of lanes, inside to outside
/// what should the semantic separator between them be
#[allow(clippy::unnecessary_wraps)]
pub(in crate::transform::tags_to_lanes) fn lane_pair_to_semantic_separator(
    lanes: [&LaneBuilder; 2],
    road: &RoadBuilder,
    tags: &Tags,
    locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Option<Separator> {
    let [inside, outside] = lanes;
    let direction_change = match [inside.direction.some(), outside.direction.some()] {
        [None | Some(Direction::Both), _] | [_, None | Some(Direction::Both)] => {
            DirectionChange::None
        },
        [Some(Direction::Forward), Some(Direction::Forward)]
        | [Some(Direction::Backward), Some(Direction::Backward)] => DirectionChange::Same,
        [Some(Direction::Forward), Some(Direction::Backward)]
        | [Some(Direction::Backward), Some(Direction::Forward)] => DirectionChange::Opposite,
    };
    match (
        [
            (inside.r#type.some(), inside.designated.some()),
            (outside.r#type.some(), outside.designated.some()),
        ],
        direction_change,
    ) {
        // Foot
        ([_, (_, Some(Designated::Foot))], _) => Some(Separator::Kerb {
            parking_condition: None,
        }),
        // Shoulder
        ([_, (Some(LaneType::Shoulder), _)], _) => Some(Separator::Shoulder {
            speed: inside.max_speed.map(SpeedClass::from),
        }),
        // Motor to motor
        ([(_, Some(Designated::Motor)), (_, Some(Designated::Motor))], _) => {
            motor_lane_pair_to_semantic_separator(
                [inside, outside],
                direction_change,
                road,
                tags,
                locale,
                warnings,
            )
        },
        // Modal separation
        ([(_, Some(inside_designated)), (_, Some(outside_designated))], _)
            if inside_designated != outside_designated =>
        {
            Some(Separator::Modal {
                speed: inside.max_speed.map(SpeedClass::from),
                change: LaneChange::default(),
                inside: inside_designated,
                outside: outside_designated,
            })
        },
        // TODO: error return
        _ => {
            warnings.push(TagsToLanesMsg::separator_unknown(
                inside.clone(),
                outside.clone(),
            ));
            None
        },
    }
}

#[allow(clippy::unnecessary_wraps)]
fn motor_lane_pair_to_semantic_separator(
    [inside, _outside]: [&LaneBuilder; 2],
    direction_change: DirectionChange,
    road: &RoadBuilder,
    _tags: &Tags,
    locale: &Locale,
    _warnings: &mut RoadWarnings,
) -> Option<Separator> {
    match road
        .lanes_ltr(locale)
        .filter(|lane| {
            matches!(lane.r#type.some(), Some(LaneType::Travel))
                && matches!(
                    lane.designated.some(),
                    Some(Designated::Motor | Designated::Bus),
                )
        })
        .count()
    {
        2 => Some(Separator::Centre {
            speed: inside.max_speed.map(SpeedClass::from),
            overtake: Overtake::default(),
            more_than_2_lanes: false,
        }),
        _ => match direction_change {
            DirectionChange::Same => Some(Separator::Lane {
                speed: inside.max_speed.map(SpeedClass::from),
                change: LaneChange::default(),
            }),
            DirectionChange::None | DirectionChange::Opposite => Some(Separator::Centre {
                speed: inside.max_speed.map(SpeedClass::from),
                overtake: Overtake::default(),
                more_than_2_lanes: true,
            }),
        },
    }
}

/// Given a pair of lanes, inside to outside
/// what should the separator between them be
#[allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
pub(in crate::transform::tags_to_lanes) fn semantic_separator_to_lane(
    [inside, outside]: [&LaneBuilder; 2],
    separator: &Separator,
    _road: &RoadBuilder,
    tags: &Tags,
    locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Option<Lane> {
    match separator {
        // Foot
        Separator::Kerb { .. } => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: Style::KerbUp,
                color: None,
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        // Shoulder
        Separator::Shoulder { .. } => {
            if tags.is("motorroad", "yes") {
                if let Some(c) = &locale.country {
                    if c == &Country::the_netherlands() {
                        return Some(Lane::Separator {
                            // https://puc.overheid.nl/rijkswaterstaat/doc/PUC_125514_31/
                            // 4.2.5 and 4.2.6
                            markings: Markings::new(vec![Marking {
                                style: Style::SolidLine,
                                color: Some(Color::White),
                                width: Some(Marking::DEFAULT_WIDTH),
                            }]),
                        });
                    }
                }
            }
            Some(Lane::Separator {
                markings: Markings::new(vec![Marking {
                    style: Style::SolidLine,
                    color: Some(Color::White),
                    width: Some(Marking::DEFAULT_WIDTH),
                }]),
            })
        },
        Separator::Centre {
            more_than_2_lanes, ..
        } => {
            if tags.is("motorroad", "yes") {
                if let Some(c) = &locale.country {
                    if c == &Country::the_netherlands() {
                        return Some(Lane::Separator {
                            // https://puc.overheid.nl/rijkswaterstaat/doc/PUC_125514_31/
                            // 4.2.5 and 4.2.6
                            markings: Markings::new(vec![
                                Marking {
                                    style: Style::BrokenLine,
                                    color: Some(Color::White),
                                    width: Some(Metre::new(0.15_f64)),
                                },
                                Marking {
                                    style: Style::SolidLine,
                                    color: Some(Color::Green),
                                    width: Some(2.0_f64 * Marking::DEFAULT_SPACE),
                                },
                                Marking {
                                    style: Style::BrokenLine,
                                    color: Some(Color::White),
                                    width: Some(Metre::new(0.15_f64)),
                                },
                            ]),
                        });
                    }
                }
            }
            if let Some(c) = &locale.country {
                if c == &Country::the_united_kingdom_of_great_britain_and_northern_ireland() {
                    return Some(Lane::Separator {
                        // https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/782724/traffic-signs-manual-chapter-03.pdf
                        // Traffic Signs Manual, Chapter 3
                        // Page 90, 9.3.3
                        markings: Markings::new(vec![Marking {
                            style: Style::BrokenLine,
                            color: Some(Color::White),
                            width: Some(Metre::new(0.100_f64)),
                        }]),
                    });
                }
            }
            warnings.push(TagsToLanesMsg::separator_locale_unused(
                inside.clone(),
                outside.clone(),
            ));
            Some(Lane::Separator {
                markings: if *more_than_2_lanes {
                    Markings::new(vec![
                        Marking {
                            style: Style::SolidLine,
                            color: Some(Color::White),
                            width: Some(Marking::DEFAULT_WIDTH),
                        },
                        Marking {
                            style: Style::NoFill,
                            color: None,
                            width: Some(Marking::DEFAULT_SPACE),
                        },
                        Marking {
                            style: Style::SolidLine,
                            color: Some(Color::White),
                            width: Some(Marking::DEFAULT_WIDTH),
                        },
                    ])
                } else {
                    Markings::new(vec![Marking {
                        style: Style::DottedLine,
                        color: Some(locale.separator_motor_color()),
                        width: Some(locale.separator_motor_width()),
                    }])
                },
            })
        },
        Separator::Lane { .. } => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: Style::DottedLine,
                color: Some(Color::White),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        // Modal separation
        Separator::Modal {
            outside: designated,
            ..
        } => {
            if let Some(c) = &locale.country {
                if c == &Country::the_united_kingdom_of_great_britain_and_northern_ireland() {
                    if designated == &Designated::Bus {
                        return Some(Lane::Separator {
                            // https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/782724/traffic-signs-manual-chapter-03.pdf
                            // Traffic Signs Manual, Chapter 3
                            // Page 90, 9.3.3
                            markings: Markings::new(vec![Marking {
                                style: Style::SolidLine,
                                color: Some(Color::White),
                                width: Some(Metre::new(0.250_f64)),
                            }]),
                        });
                    }
                    if designated == &Designated::Bicycle {
                        return Some(Lane::Separator {
                            // https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/782724/traffic-signs-manual-chapter-03.pdf
                            // Traffic Signs Manual, Chapter 3
                            // Page 90, 9.3.3
                            markings: Markings::new(vec![Marking {
                                style: Style::SolidLine,
                                color: Some(Color::White),
                                width: Some(Metre::new(0.150_f64)),
                            }]),
                        });
                    }
                }
            }
            warnings.push(TagsToLanesMsg::separator_locale_unused(
                inside.clone(),
                outside.clone(),
            ));
            Some(Lane::Separator {
                markings: Markings::new(vec![Marking {
                    style: Style::SolidLine,
                    color: Some(Color::White),
                    width: Some(Marking::DEFAULT_WIDTH),
                }]),
            })
        },
        // TODO: error return
        _ => {
            warnings.push(TagsToLanesMsg::separator_unknown(
                inside.clone(),
                outside.clone(),
            ));
            Some(Lane::Separator {
                markings: Markings::new(vec![Marking {
                    style: Style::BrokenLine,
                    color: Some(Color::Red),
                    width: Some(Marking::DEFAULT_WIDTH),
                }]),
            })
        },
    }
}

/// Given a lane on the outer edge of a way
/// what should the separator be.
/// Lanes are defined inside to outside
#[allow(clippy::unnecessary_wraps)]
pub(super) fn outer_edge_semantic_separator(
    lane: &LaneBuilder,
    tags: &Tags,
    locale: &Locale,
) -> Option<EdgeSeparator> {
    if lane.r#type.some() == Some(LaneType::Travel) {
        if let Some(c) = &locale.country {
            if c == &Country::the_united_kingdom_of_great_britain_and_northern_ireland()
                && tags.is("parking:condition:both", "no_stopping")
            {
                return Some(EdgeSeparator::Hard {
                    parking_condition: Some(ParkingCondition::NoStopping),
                });
            }
        }
    }
    None
}

/// Given a pair of lanes, inside to outside
/// what should the separator between them be
#[allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
pub(in crate::transform::tags_to_lanes) fn semantic_edge_separator_to_lane(
    separator: &EdgeSeparator,
    _road: &RoadBuilder,
    _tags: &Tags,
    _locale: &Locale,
    _warnings: &mut RoadWarnings,
) -> Option<Lane> {
    match separator {
        EdgeSeparator::Hard { .. } => Some(Lane::Separator {
            markings: Markings::new(vec![
                Marking {
                    style: Style::SolidLine,
                    color: Some(Color::Red),
                    width: Some(Metre::new(0.100)),
                },
                Marking {
                    style: Style::NoFill,
                    color: None,
                    width: Some(Metre::new(0.080)),
                },
                Marking {
                    style: Style::SolidLine,
                    color: Some(Color::Red),
                    width: Some(Metre::new(0.100)),
                },
            ]),
        }),
    }
}

/// Given a lane on the inner edge of a way
/// what should the separator be.
/// Lanes are defined inside to outside
#[allow(clippy::unnecessary_wraps)]
pub(super) fn lane_to_inner_edge_separator(_lane: &LaneBuilder) -> Option<Lane> {
    Some(Lane::Separator {
        markings: Markings::new(vec![Marking {
            style: Style::SolidLine,
            color: Some(Color::White),
            width: Some(Marking::DEFAULT_WIDTH),
        }]),
    })
}
