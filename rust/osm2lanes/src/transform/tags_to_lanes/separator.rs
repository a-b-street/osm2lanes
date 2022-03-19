use celes::Country;

use super::*;
use crate::road::Markings;

#[derive(Clone, Copy)]
enum DirectionChange {
    None,
    Same,
    Opposite,
}

/// Given a pair of lanes, inside to outside
/// what should the separator between them be
pub(super) fn lanes_to_separator(
    lanes: [&LaneBuilder; 2],
    road: &RoadBuilder,
    tags: &Tags,
    locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Option<Lane> {
    let [inside, outside] = lanes;
    let direction_change = match [inside.direction.some(), outside.direction.some()] {
        [None | Some(LaneDirection::Both), _] | [_, None | Some(LaneDirection::Both)] => {
            DirectionChange::None
        }
        [Some(LaneDirection::Forward), Some(LaneDirection::Forward)]
        | [Some(LaneDirection::Backward), Some(LaneDirection::Backward)] => DirectionChange::Same,
        [Some(LaneDirection::Forward), Some(LaneDirection::Backward)]
        | [Some(LaneDirection::Backward), Some(LaneDirection::Forward)] => {
            DirectionChange::Opposite
        }
    };
    match (
        [
            (inside.r#type.some(), inside.designated.some()),
            (outside.r#type.some(), outside.designated.some()),
        ],
        direction_change,
    ) {
        // Foot
        ([_, (_, Some(LaneDesignated::Foot))], _) => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::KerbUp,
                color: None,
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        // Shoulder
        ([_, (Some(LaneType::Shoulder), _)], _) => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::SolidLine,
                color: Some(MarkingColor::White),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        ([(_, Some(LaneDesignated::Motor)), (_, Some(LaneDesignated::Motor))], _) => {
            motor_lanes_to_separator(
                [inside, outside],
                direction_change,
                road,
                tags,
                locale,
                warnings,
            )
        }
        // Modal separation
        ([(_, inside_designated), (_, outside_designated)], _)
            if inside_designated != outside_designated =>
        {
            warnings.push(RoadMsg::SeparatorLocaleUnused {
                inside: inside.clone(),
                outside: outside.clone(),
            });
            Some(Lane::Separator {
                markings: Markings::new(vec![Marking {
                    style: MarkingStyle::SolidLine,
                    color: Some(MarkingColor::White),
                    width: Some(Marking::DEFAULT_WIDTH),
                }]),
            })
        }
        // TODO: error return
        _ => {
            warnings.push(RoadMsg::SeparatorUnknown {
                inside: inside.clone(),
                outside: outside.clone(),
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
    [inside, outside]: [&LaneBuilder; 2],
    direction_change: DirectionChange,
    road: &RoadBuilder,
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
    warnings.push(RoadMsg::SeparatorLocaleUnused {
        inside: inside.clone(),
        outside: outside.clone(),
    });
    match road
        .lanes_ltr(locale)
        .filter(|lane| {
            matches!(lane.r#type.some(), Some(LaneType::Travel))
                && matches!(
                    lane.designated.some(),
                    Some(LaneDesignated::Motor | LaneDesignated::Bus),
                )
        })
        .count()
    {
        2 => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::DottedLine,
                color: Some(MarkingColor::White),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
        _ => match direction_change {
            DirectionChange::Same => Some(Lane::Separator {
                markings: Markings::new(vec![Marking {
                    style: MarkingStyle::DottedLine,
                    color: Some(MarkingColor::White),
                    width: Some(Marking::DEFAULT_WIDTH),
                }]),
            }),
            DirectionChange::None | DirectionChange::Opposite => Some(Lane::Separator {
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
            }),
        },
    }
}

/// Given a lane on the edge of a way
/// what should the separator be
pub(super) fn lane_to_edge_separator(_lane: &LaneBuilder) -> Option<Lane> {
    None
}
