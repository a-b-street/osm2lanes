use celes::Country;

use super::*;
use crate::road::Markings;

/// Given a pair of lanes
/// what should the separator between them be
pub(super) fn lanes_to_separator(
    lanes: &[LaneBuilder; 2],
    road: &RoadBuilder,
    tags: &Tags,
    locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Option<Lane> {
    match lanes {
        [left, right] => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::KerbDown,
                color: None,
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        [_, Lane::Travel {
            designated: LaneDesignated::Foot,
            ..
        }] => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::KerbUp,
                color: None,
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        [Lane::Shoulder { .. }, _] | [_, Lane::Shoulder { .. }] => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::SolidLine,
                color: Some(MarkingColor::White),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        [Lane::Travel {
            designated: LaneDesignated::Motor,
            ..
        }
        | Lane::Travel {
            designated: LaneDesignated::Bus,
            ..
        }, Lane::Travel {
            designated: LaneDesignated::Bicycle,
            ..
        }]
        | [Lane::Travel {
            designated: LaneDesignated::Bicycle,
            ..
        }, Lane::Travel {
            designated: LaneDesignated::Motor,
            ..
        }
        | Lane::Travel {
            designated: LaneDesignated::Bus,
            ..
        }] => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::SolidLine,
                color: Some(MarkingColor::White),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        [left @ Lane::Travel {
            designated: LaneDesignated::Motor,
            ..
        }, right @ Lane::Travel {
            designated: LaneDesignated::Motor,
            ..
        }] => motor_lanes_to_separator(road, left, right, tags, locale, warnings),
        // TODO: error return
        [left, right] => {
            warnings.push(RoadMsg::Unimplemented {
                description: Some(format!("lane separators for {:?} and {:?}", left, right)),
                tags: None,
            });
            Some(Lane::Separator {
                markings: Markings::new(vec![Marking {
                    style: MarkingStyle::BrokenLine,
                    color: Some(MarkingColor::Red),
                    width: Some(Marking::DEFAULT_WIDTH),
                }]),
            })
        }
    }
}

fn motor_lanes_to_separator(
    road: &[Lane],
    left: &Lane,
    right: &Lane,
    tags: &Tags,
    locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Option<Lane> {
    if tags.is("motorroad", "yes") {
        if let Some(c) = &locale.country {
            if c == &Country::the_netherlands() {
                return Some(Lane::Separator {
                    markings: Markings::new(vec![
                        Marking {
                            style: MarkingStyle::BrokenLine,
                            color: Some(MarkingColor::White),
                            width: Some(Marking::DEFAULT_WIDTH),
                        },
                        Marking {
                            style: MarkingStyle::SolidLine,
                            color: Some(MarkingColor::Green),
                            width: Some(2.0 * Marking::DEFAULT_SPACE),
                        },
                        Marking {
                            style: MarkingStyle::BrokenLine,
                            color: Some(MarkingColor::White),
                            width: Some(Marking::DEFAULT_WIDTH),
                        },
                    ]),
                });
            }
        }
    }
    warnings.push(RoadMsg::Unimplemented {
        description: Some(format!(
            "lane separators for {:?} and {:?}, using default",
            left, right
        )),
        tags: None,
    });
    match road
        .iter()
        .filter(|lane| lane.is_motor() || lane.is_bus())
        .count()
    {
        2 => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::DottedLine,
                color: Some(MarkingColor::White),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        _ => {
            if left.direction() == right.direction() {
                Some(Lane::Separator {
                    markings: Markings::new(vec![Marking {
                        style: MarkingStyle::DottedLine,
                        color: Some(MarkingColor::White),
                        width: Some(Marking::DEFAULT_WIDTH),
                    }]),
                })
            } else {
                Some(Lane::Separator {
                    markings: Markings::new(vec![
                        Marking {
                            style: MarkingStyle::SolidLine,
                            color: Some(MarkingColor::White),
                            width: Some(Marking::DEFAULT_WIDTH),
                        },
                        Marking {
                            style: MarkingStyle::NoFill,
                            color: None,
                            width: Some(Marking::DEFAULT_SPACE),
                        },
                        Marking {
                            style: MarkingStyle::SolidLine,
                            color: Some(MarkingColor::White),
                            width: Some(Marking::DEFAULT_WIDTH),
                        },
                    ]),
                })
            }
        }
    }
}

/// Given a lane on the edge of a way
/// what should the separator be
pub(super) fn lane_to_edge_separator(_lane: &LaneBuilder) -> Option<Lane> {
    None
}
