use super::{Infer, Oneway};
use crate::locale::Locale;
use crate::road::Direction;
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::modes::{BuswayScheme, LanesBusScheme};
use crate::transform::{RoadWarnings, TagsToLanesMsg};

const LANES: TagKey = TagKey::from("lanes");
#[derive(Debug)]
pub struct LanesScheme {
    pub lanes: Infer<usize>,
    pub forward: Infer<usize>,
    pub backward: Infer<usize>,
    pub bothways: Infer<usize>,
}

impl LanesScheme {
    /// Parses and validates the `lanes` scheme (which excludes parking lanes, bike lanes, etc.).
    /// See <https://wiki.openstreetmap.org/wiki/Key:lanes>.
    ///
    /// Validates `lanes[:{forward,both_ways,backward}]=*` and `centre_turn_lane=yes`.
    #[allow(
        clippy::integer_arithmetic,
        clippy::integer_division,
        clippy::too_many_lines
    )]
    pub(super) fn new(
        tags: &Tags,
        oneway: Oneway,
        centre_turn_lane: &CentreTurnLaneScheme, // TODO prefer TurnLanesScheme
        busway: &BuswayScheme,
        lanes_bus: &LanesBusScheme,
        _locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        // The tags for this schema (which we will validate).
        let tagged_lanes: Option<usize> = tags.get_parsed(&LANES, warnings);
        let tagged_forward: Option<usize> = tags.get_parsed(&(LANES + "forward"), warnings);
        let tagged_backward: Option<usize> = tags.get_parsed(&(LANES + "backward"), warnings);
        let tagged_bothways: Option<usize> = tags.get_parsed(&(LANES + "both_ways"), warnings);

        // Calculate the bothways lanes.
        let bothways = match (tagged_bothways, centre_turn_lane.present.some()) {
            (Some(bw), _) => Infer::Direct(bw),
            (None, Some(true)) => Infer::Calculated(1),
            (None, Some(false)) => Infer::Calculated(0),
            (None, None) => Infer::Default(0),
        };
        let bothway_lanes = bothways.some().unwrap_or(0);
        // Check it against the centre turn lane tag.
        if let (Infer::Direct(bw), Infer::Direct(t)) = (bothways, centre_turn_lane.present) {
            // TODO what if the values conflict but are not Direct? Might not ever happen.
            if (!t && bw > 0) || (t && bw == 0) {
                warnings.push(TagsToLanesMsg::ambiguous_tags(
                    tags.subset(&[LANES + "both_ways", CENTRE_TURN_LANE]),
                ));
            }
        }

        let forward_bus_lanes = lanes_bus.forward.some().unwrap_or(0);
        let backward_bus_lanes = lanes_bus.backward.some().unwrap_or(0);

        // TODO: after calculating the lanes scheme here (sometimes using bus lanes to guess),
        // check that bus lanes don't conflict (if we didn't guess).

        if oneway.into() {
            // Ignore lanes:{both_ways,backward}=
            // TODO ignore oneway instead?
            if tagged_bothways.is_some() || tagged_backward.is_some() {
                warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                    "oneway",
                    "lanes:both_ways",
                    "lanes:backward",
                ])));
            }

            if let Some(l) = tagged_lanes {
                let mut result = Self {
                    lanes: Infer::Direct(l),
                    forward: Infer::Calculated(l),
                    backward: Infer::Default(0),
                    bothways,
                };
                // Roads with car traffic in one direction and bus traffic in the other, can be
                // tagged `oneway=yes` `busway:<backward>=opposite_lane` but are more "canonically"
                // tagged `oneway=no` `lanes:backward=1` `busway:<backward>=lane`.
                if let Some(Some(Direction::Backward)) = busway.backward_side_direction.some() {
                    result.forward = Infer::Calculated(l - 1);
                    result.backward = Infer::Calculated(1);
                }

                if tagged_forward.map_or(false, |f| f != result.forward.some().unwrap()) {
                    // TODO What is the right warning for straight up conflicts in tag values?
                    warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                        "oneway",
                        "lanes",
                        "lanes:forward",
                    ])));
                }
                result
            } else if let Some(f) = tagged_forward {
                Self {
                    lanes: Infer::Calculated(f),
                    forward: Infer::Direct(f),
                    backward: Infer::Default(0),
                    bothways,
                }
            } else {
                // Assume 1 lane, but guess 1 normal lane plus bus lanes.
                let assumed_forward = 1; // TODO depends on highway tag
                if forward_bus_lanes > 0 {
                    Self {
                        lanes: Infer::Guessed(assumed_forward + forward_bus_lanes),
                        forward: Infer::Guessed(assumed_forward + forward_bus_lanes),
                        backward: Infer::Default(0),
                        bothways,
                    }
                } else {
                    Self {
                        lanes: Infer::Default(assumed_forward),
                        forward: Infer::Default(assumed_forward),
                        backward: Infer::Default(0),
                        bothways,
                    }
                }
            }
        } else {
            // Twoway
            match (tagged_lanes, tagged_forward, tagged_backward) {
                (Some(l), Some(f), Some(b)) => {
                    if l != f + b + bothway_lanes {
                        warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                            "lanes",
                            "lanes:forward",
                            "lanes:both_ways",
                            "lanes:backward",
                        ])));
                    }
                    Self {
                        lanes: Infer::Direct(l),
                        forward: Infer::Direct(f),
                        backward: Infer::Direct(b),
                        bothways,
                    }
                },
                (None, Some(f), Some(b)) => Self {
                    lanes: Infer::Calculated(f + b + bothway_lanes),
                    forward: Infer::Direct(f),
                    backward: Infer::Direct(b),
                    bothways,
                },
                (None, _, _) => {
                    // Without the "lanes" tag, assume one normal lane in each dir, plus bus lanes.
                    let f = tagged_forward.unwrap_or(1 + forward_bus_lanes);
                    let b = tagged_backward.unwrap_or(1 + backward_bus_lanes);
                    let forward = match (tagged_forward.is_some(), forward_bus_lanes) {
                        (true, _) => Infer::Direct(f),
                        (false, 0) => Infer::Default(f),
                        (false, _) => Infer::Guessed(f),
                    };
                    let backward = match (tagged_backward.is_some(), backward_bus_lanes) {
                        (true, _) => Infer::Direct(b),
                        (false, 0) => Infer::Default(b),
                        (false, _) => Infer::Guessed(b),
                    };
                    let lanes = Infer::Default(f + b + bothway_lanes);
                    // TODO lanes.downgrade(&[forward, backward, bothways]);
                    Self {
                        lanes,
                        forward,
                        backward,
                        bothways,
                    }
                },
                (Some(l), Some(f), None) => Self {
                    lanes: Infer::Direct(l),
                    forward: Infer::Direct(f),
                    backward: Infer::Calculated(l - f - bothway_lanes),
                    bothways,
                },
                (Some(l), None, Some(b)) => Self {
                    lanes: Infer::Direct(l),
                    forward: Infer::Calculated(l - b - bothway_lanes),
                    backward: Infer::Direct(b),
                    bothways,
                },
                // Alleyways or narrow unmarked roads, usually:
                (Some(1), None, None) => Self {
                    lanes: Infer::Direct(1),
                    forward: Infer::Default(0),
                    backward: Infer::Default(0),
                    bothways: Infer::Guessed(1),
                },
                (Some(l), None, None) => {
                    if l % 2 == 0 && centre_turn_lane.present.some().unwrap_or(false) {
                        // Only tagged with lanes and deprecated center_turn_lane tag.
                        // Assume the center_turn_lane is in addition to evenly divided lanes.
                        Self {
                            lanes: Infer::Calculated(l + 1),
                            forward: Infer::Guessed(l / 2),
                            backward: Infer::Guessed(l / 2),
                            bothways: Infer::Calculated(1),
                        }
                    } else {
                        // Distribute normal lanes evenly.
                        let remaining_lanes =
                            l - bothway_lanes - forward_bus_lanes - backward_bus_lanes;
                        if remaining_lanes % 2 != 0 {
                            warnings.push(TagsToLanesMsg::ambiguous_str("Total lane count cannot be evenly divided between the forward and backward"));
                        }
                        let half = (remaining_lanes + 1) / 2; // usize division rounded up.
                        Self {
                            lanes: Infer::Direct(l),
                            forward: Infer::Guessed(half + forward_bus_lanes),
                            backward: Infer::Guessed(
                                remaining_lanes - half - bothway_lanes + backward_bus_lanes,
                            ),
                            bothways,
                        }
                    }
                },
            }
        }
    }
}

// struct TurnLanesScheme {
//     lanes: Vec<Option<TurnDirections>>,
// }
// struct TurnDirections {
//     through: bool,
//     left: bool,
//     right: bool,
//     slight_left: bool,
//     slight_right: bool,
//     merge_left: bool,
//     merge_right: bool,
// }

const CENTRE_TURN_LANE: TagKey = TagKey::from("centre_turn_lane");
pub struct CentreTurnLaneScheme {
    pub present: Infer<bool>,
}
impl CentreTurnLaneScheme {
    /// Parses and validates the `centre_turn_lane` tag and emits a deprecation warning.
    /// See <https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane>.
    pub(super) fn new(
        tags: &Tags,
        _oneway: Oneway,
        _locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        let present = match tags.get(CENTRE_TURN_LANE) {
            None => Infer::Default(false),
            Some("yes") => {
                warnings.push(TagsToLanesMsg::deprecated_tags(
                    tags.subset(&[CENTRE_TURN_LANE]),
                ));
                Infer::Direct(true)
            },
            Some("no") => {
                warnings.push(TagsToLanesMsg::deprecated_tag(CENTRE_TURN_LANE, ""));
                Infer::Direct(false)
            },
            Some(_) => {
                warnings.push(TagsToLanesMsg::deprecated_tag(CENTRE_TURN_LANE, ""));
                // TODO what's the right warning for bad tag values?
                warnings.push(TagsToLanesMsg::unsupported_tags(
                    tags.subset(&[CENTRE_TURN_LANE]),
                ));
                Infer::Guessed(false)
            },
        };
        Self { present }
    }
}
