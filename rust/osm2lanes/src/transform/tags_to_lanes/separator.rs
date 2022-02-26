use super::*;
use crate::road::Markings;

#[allow(clippy::needless_collect)]
pub fn insert_separators(lanes: Vec<Lane>) -> Result<Vec<Lane>, RoadError> {
    let left_edge = lane_to_edge_separator(lanes.first().unwrap());
    let right_edge = lane_to_edge_separator(lanes.first().unwrap());
    let separators: Vec<Option<Lane>> = lanes
        .windows(2)
        .map(|window| lanes_to_separator(window.try_into().unwrap(), &lanes))
        .collect();
    // I promise this is good code, but it might need a little explanation.
    // If there are n lanes, there will be 1 + (n-1) + 1 separators.
    // We interleave (zip(n+1)+flat_map) the separators with the lanes, and flatten to remove the Nones.
    let lanes: Vec<Lane> = iter::once(left_edge)
        .chain(separators.into_iter())
        .chain(iter::once(right_edge))
        .zip(lanes.into_iter().map(Some).chain(iter::once(None)))
        .flat_map(|(a, b)| [a, b])
        .flatten()
        .collect();
    Ok(lanes)
}

/// Given a pair of lanes
/// what should the separator between them be
fn lanes_to_separator(lanes: &[Lane; 2], road: &[Lane]) -> Option<Lane> {
    match lanes {
        [Lane::Travel {
            designated: LaneDesignated::Foot,
            ..
        }, _] => Some(Lane::Separator {
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
        }] => {
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
        // TODO: error return
        _ => Some(Lane::Separator {
            markings: Markings::new(vec![Marking {
                style: MarkingStyle::BrokenLine,
                color: Some(MarkingColor::Red),
                width: Some(Marking::DEFAULT_WIDTH),
            }]),
        }),
    }
}

/// Given a lane on the edge of a way
/// what should the separator be
fn lane_to_edge_separator(_lane: &Lane) -> Option<Lane> {
    None
}
