use super::{Infer, Oneway, TagsNumeric};
use crate::locale::Locale;
use crate::tag::{Highway, TagKey, Tags};
use crate::transform::tags_to_lanes::modes::BusLaneCount;
use crate::transform::{RoadWarnings, TagsToLanesMsg};

/// The number of lanes for motor vehicle traffic
#[derive(Debug)]
pub enum Counts {
    One, // One bidirectional lane
    Directional {
        forward: Infer<usize>,
        backward: Infer<usize>,
        centre_turn_lane: Infer<bool>,
    },
}

impl Counts {
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
        highway: &Highway,
        centre_turn_lane: &CentreTurnLaneScheme, // TODO prefer TurnLanesScheme
        bus: &BusLaneCount,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        let lanes = LanesDirectionScheme::from_tags(tags, oneway, locale, warnings);

        let centre_turn_lane = match (lanes.both_ways, centre_turn_lane.some()) {
            (Some(()), None | Some(true)) => Infer::Direct(true),
            (None, Some(true)) => Infer::Calculated(true),
            (None, Some(false)) => Infer::Calculated(false),
            (None, None) => Infer::Default(false),
            (Some(()), Some(false)) => {
                warnings.push(TagsToLanesMsg::ambiguous_tags(
                    tags.subset(&[LANES + "both_ways", CENTRE_TURN_LANE]),
                ));
                Infer::Default(true)
            },
        };
        let both_ways: usize = if centre_turn_lane.some().unwrap_or(false) {
            1
        } else {
            0
        };

        // TODO: after calculating the lanes scheme here (sometimes using bus lanes to guess),
        // check that bus lanes don't conflict (if we didn't guess).

        if oneway.into() {
            // Ignore lanes:{both_ways,backward}=
            // TODO ignore oneway instead?
            if lanes.both_ways.is_some() || lanes.backward.is_some() {
                warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset([
                    "oneway",
                    "lanes:both_ways",
                    "lanes:backward",
                ])));
            }

            if let Some(total) = lanes.total {
                // Total lanes probably all going forward.
                // Roads with car traffic in one direction and bus traffic in the other, can be
                // tagged `oneway=yes` `busway:<backward>=opposite_lane` but are more "canonically"
                // tagged `oneway=no` `lanes:backward=1` `busway:<backward>=lane`.
                let forward = total - both_ways - bus.backward;
                let result = Self::Directional {
                    forward: Infer::Calculated(forward),
                    backward: Infer::Calculated(bus.backward),
                    centre_turn_lane,
                };

                // TODO, shouldn't we trust the tagged value more?
                // TODO, what about backward?
                if lanes.forward.map_or(false, |direct| direct != forward) {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset([
                        "oneway",
                        "lanes",
                        "lanes:forward",
                    ])));
                }

                result
            } else if let Some(f) = lanes.forward {
                Self::Directional {
                    forward: Infer::Direct(f),
                    backward: Infer::Default(0),
                    centre_turn_lane,
                }
            } else {
                // Assume 1 lane, but guess 1 normal lane plus bus lanes.
                let assumed_forward = 1; // TODO depends on highway tag
                Self::Directional {
                    forward: Infer::Default(assumed_forward + bus.forward),
                    backward: Infer::Default(0),
                    centre_turn_lane,
                }
            }
        } else {
            // Twoway
            match (lanes.total, lanes.forward, lanes.backward) {
                (Some(l), Some(f), Some(b)) => {
                    if l != f + b + both_ways {
                        warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset([
                            "lanes",
                            "lanes:forward",
                            "lanes:backward",
                            "lanes:both_ways",
                            "center_turn_lanes",
                        ])));
                    }
                    Self::Directional {
                        forward: Infer::Direct(f),
                        backward: Infer::Direct(b),
                        centre_turn_lane,
                    }
                },
                (None, Some(f), Some(b)) => Self::Directional {
                    forward: Infer::Direct(f),
                    backward: Infer::Direct(b),
                    centre_turn_lane,
                },
                (Some(l), Some(f), None) => Self::Directional {
                    forward: Infer::Direct(f),
                    backward: Infer::Calculated(l - f - both_ways),
                    centre_turn_lane,
                },
                (Some(l), None, Some(b)) => Self::Directional {
                    forward: Infer::Calculated(l - b - both_ways),
                    backward: Infer::Direct(b),
                    centre_turn_lane,
                },
                // Alleyways or narrow unmarked roads, usually:
                (Some(1), None, None) => Self::One,
                (Some(l), None, None) => {
                    if l % 2 == 0 && centre_turn_lane.some().unwrap_or(false) {
                        // Only tagged with lanes and deprecated center_turn_lane tag.
                        // Assume the center_turn_lane is in addition to evenly divided lanes.
                        Self::Directional {
                            forward: Infer::Default(l / 2),
                            backward: Infer::Default(l / 2),
                            centre_turn_lane,
                        }
                    } else {
                        // Distribute normal lanes evenly.
                        let remaining_lanes = l - both_ways - bus.forward - bus.backward;
                        if remaining_lanes % 2 != 0 {
                            warnings.push(TagsToLanesMsg::ambiguous_str("Total lane count cannot be evenly divided between the forward and backward"));
                        }
                        let half = (remaining_lanes + 1) / 2; // usize division rounded up.
                        Self::Directional {
                            forward: Infer::Default(half + bus.forward),
                            backward: Infer::Default(
                                remaining_lanes - half - both_ways + bus.backward,
                            ),
                            centre_turn_lane,
                        }
                    }
                },
                (None, None, None) => {
                    if locale.has_split_lanes(highway.r#type())
                        || bus.forward > 0
                        || bus.backward > 0
                    {
                        Self::Directional {
                            forward: Infer::Default(1 + bus.forward),
                            backward: Infer::Default(1 + bus.backward),
                            centre_turn_lane,
                        }
                    } else {
                        Self::One
                    }
                },
                (None, _, _) => {
                    if locale.has_split_lanes(highway.r#type()) {
                        // Without the "lanes" tag, assume one normal lane in each dir, plus bus lanes.
                        let forward = Infer::from(lanes.forward).or_default(1 + bus.forward);
                        let backward = Infer::from(lanes.backward).or_default(1 + bus.forward);
                        // TODO lanes.downgrade(&[forward, backward, bothways]);
                        Self::Directional {
                            forward,
                            backward,
                            centre_turn_lane,
                        }
                    } else {
                        Self::One
                    }
                },
            }
        }
    }
}

const LANES: TagKey = TagKey::from_static("lanes");

/// `lanes` and directional `lanes:*` scheme, see <https://wiki.openstreetmap.org/wiki/Key:lanes>
pub(in crate::transform::tags_to_lanes) struct LanesDirectionScheme {
    total: Option<usize>,
    forward: Option<usize>,
    backward: Option<usize>,
    both_ways: Option<()>,
}
impl LanesDirectionScheme {
    pub fn from_tags(
        tags: &Tags,
        _oneway: Oneway,
        _locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        let both_ways = tags
            .get_parsed(&(LANES + "both_ways"), warnings)
            .filter(|&v: &usize| {
                if v == 1 {
                    true
                } else {
                    warnings.push(TagsToLanesMsg::unsupported(
                        "lanes:both_ways must be 1",
                        tags.subset(&[LANES + "both_ways"]),
                    ));
                    false
                }
            })
            .map(|_v| {});
        Self {
            total: tags.get_parsed(&LANES, warnings),
            forward: tags.get_parsed(&(LANES + "forward"), warnings),
            backward: tags.get_parsed(&(LANES + "backward"), warnings),
            both_ways,
        }
    }
}

const CENTRE_TURN_LANE: TagKey = TagKey::from_static("centre_turn_lane");
pub(in crate::transform::tags_to_lanes) struct CentreTurnLaneScheme(pub Option<bool>);
impl CentreTurnLaneScheme {
    /// Parses and validates the `centre_turn_lane` tag and emits a deprecation warning.
    /// See <https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane>.
    pub fn from_tags(
        tags: &Tags,
        _oneway: Oneway,
        _locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        if let Some(v) = tags.get(&CENTRE_TURN_LANE) {
            warnings.push(TagsToLanesMsg::deprecated_tags(
                tags.subset(&[CENTRE_TURN_LANE]),
            ));
            match v {
                "yes" => Self(Some(true)),
                "no" => Self(Some(false)),
                _ => {
                    warnings.push(TagsToLanesMsg::unsupported_tags(
                        tags.subset([&CENTRE_TURN_LANE]),
                    ));
                    Self(None)
                },
            }
        } else {
            Self(None)
        }
    }

    pub fn some(&self) -> Option<bool> {
        self.0
    }
}
