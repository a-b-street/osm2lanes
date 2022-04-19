use crate::locale::{DrivingSide, Locale};
use crate::road::Designated;
use crate::tag::{TagKey, Tags};
use crate::transform::tags_to_lanes::{Infer, Oneway, RoadBuilder};
use crate::transform::{RoadWarnings, TagsToLanesMsg};

const LANES: TagKey = TagKey::from("lanes");
const ONEWAY: TagKey = TagKey::from("oneway");

#[derive(Debug)]
pub(in crate::transform::tags_to_lanes) struct Counts {
    pub lanes: Infer<usize>,
    pub forward: Infer<usize>,
    pub backward: Infer<usize>,
    pub both_ways: Infer<usize>,
}

impl Counts {
    /// Parses and validates the `lanes` scheme (which excludes parking lanes, bike lanes, etc.).
    /// See <https://wiki.openstreetmap.org/wiki/Key:lanes>.
    ///
    /// Uses data from tags like `lanes[:{forward,both_ways,backward}]=*` and `centre_turn_lane=yes`.
    /// Tags with pipe separated access strings are also used.
    /// Lanes already existing in `road` used to determine the minima (e.g. for busway).
    #[allow(
        clippy::integer_arithmetic,
        clippy::integer_division,
        clippy::too_many_lines
    )]
    pub(in crate::transform::tags_to_lanes) fn new(
        tags: &Tags,
        road: &RoadBuilder,
        locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        // TODO, crosscheck access against tagged
        let access = Self::get_counts_by_access(tags, warnings);

        // Pre-existing lanes
        let pre_forward = road
            .forward_lanes
            .iter()
            .filter(|l| {
                matches!(
                    l.designated.some(),
                    Some(Designated::Bus | Designated::Motor)
                )
            })
            .count();
        let pre_backward = road
            .backward_lanes
            .iter()
            .filter(|l| {
                matches!(
                    l.designated.some(),
                    Some(Designated::Bus | Designated::Motor)
                )
            })
            .count();

        // The tags for this schema (which we will validate).
        let tagged_lanes: Option<usize> = tags
            .get_parsed(&LANES, warnings)
            .or_else(|| access.lanes.some());
        let tagged_forward: Option<usize> = tags
            .get_parsed(&(LANES + "forward"), warnings)
            .or_else(|| access.forward.some());
        let tagged_backward: Option<usize> = tags
            .get_parsed(&(LANES + "backward"), warnings)
            .or_else(|| access.backward.some());
        let tagged_both_ways: Option<usize> = tags
            .get_parsed(&(LANES + "both_ways"), warnings)
            .or_else(|| access.both_ways.some());

        let centre_turn_lane = CentreTurnLane::new(tags, road.oneway, locale, warnings);

        // Calculate the both_ways lanes.
        let both_ways = match (tagged_both_ways, centre_turn_lane.0) {
            (Some(bw), None) => Infer::Direct(bw),
            (Some(bw), Some(ctl)) => {
                // TODO what if the values conflict but are not Direct? Might not ever happen.
                if (bw > 0 && !ctl) || (bw == 0 && ctl) {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(
                        tags.subset(&[LANES + "both_ways", CENTRE_TURN_LANE]),
                    ));
                }
                Infer::Direct(bw)
            },
            (None, Some(true)) => Infer::Calculated(1),
            (None, Some(false)) => Infer::Calculated(0),
            (None, None) => Infer::Default(0),
        };
        let both_way_lanes = both_ways.some().unwrap_or(0);

        let counts = if road.oneway.into() {
            // Ignore lanes:{both_ways,backward}=
            if tagged_backward.is_some()
                || tagged_both_ways.is_some()
                || centre_turn_lane.0.is_some()
            {
                log::trace!("both_ways={both_ways:?} tagged_backward={tagged_backward:?}");
                warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                    ONEWAY,
                    LANES + "both_ways",
                    LANES + "backward",
                    CENTRE_TURN_LANE,
                ])));
            }

            if let (Some(l), Some(f)) = (tagged_lanes, tagged_forward) {
                // TODO What is the right warning for straight up conflicts in tag values?
                if l != f {
                    warnings.push(TagsToLanesMsg::ambiguous(
                        &format!("{l}!={f}"),
                        tags.subset(&[ONEWAY, LANES, LANES + "forward"]),
                    ));
                }
            }

            let assumed_forward = 1; // TODO depends on highway tag
            Self {
                lanes: tagged_lanes.map_or_else(
                    || tagged_forward.map_or(Infer::Default(assumed_forward), Infer::Direct),
                    Infer::Direct,
                ),
                forward: tagged_forward.map_or_else(
                    || {
                        tagged_lanes.map_or(Infer::Default(assumed_forward), |l| {
                            Infer::Direct(
                                l.checked_sub(pre_backward)
                                    .expect("road already has too many backward lanes"),
                            )
                        })
                    },
                    Infer::Direct,
                ),
                backward: tagged_backward.map_or(Infer::Default(pre_backward), Infer::Direct),
                both_ways,
            }
        } else {
            // Not oneway
            match (tagged_lanes, tagged_forward, tagged_backward) {
                (Some(l), Some(f), Some(b)) => {
                    if l != f + b + both_way_lanes {
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
                        both_ways,
                    }
                },
                (None, Some(f), Some(b)) => Self {
                    lanes: Infer::Calculated(f + b + both_way_lanes),
                    forward: Infer::Direct(f),
                    backward: Infer::Direct(b),
                    both_ways,
                },
                (Some(l), Some(f), None) => Self {
                    lanes: Infer::Direct(l),
                    forward: Infer::Direct(f),
                    backward: Infer::Calculated(
                        l.checked_sub(f)
                            .expect("too many forward lanes")
                            .checked_sub(both_way_lanes)
                            .expect(" too many both way lanes"),
                    ),
                    both_ways,
                },
                (Some(l), None, Some(b)) => Self {
                    lanes: Infer::Direct(l),
                    forward: Infer::Calculated(
                        l.checked_sub(b)
                            .expect("too many backward lanes")
                            .checked_sub(both_way_lanes)
                            .expect("too many both way lanes"),
                    ),
                    backward: Infer::Direct(b),
                    both_ways,
                },
                // Alleyways or narrow unmarked roads, usually:
                (Some(1), None, None) => Self {
                    lanes: Infer::Direct(1),
                    forward: Infer::Default(0),
                    backward: Infer::Default(0),
                    both_ways: Infer::Default(1),
                },
                (Some(l), None, None) => {
                    if l % 2 == 0 && centre_turn_lane.0.unwrap_or(false) {
                        // Only tagged with lanes and deprecated center_turn_lane tag.
                        // Assume the center_turn_lane is in addition to evenly divided lanes.
                        Self {
                            lanes: Infer::Calculated(l + 1),
                            forward: Infer::Default(l / 2),
                            backward: Infer::Default(l / 2),
                            both_ways: Infer::Calculated(1),
                        }
                    } else {
                        // Distribute normal lanes evenly.
                        let remaining_lanes = l.checked_sub(both_way_lanes).unwrap();
                        if remaining_lanes % 2 != 0 {
                            warnings.push(TagsToLanesMsg::ambiguous("Total lane count cannot be evenly divided between the forward and backward",
                                tags.subset(&[
                                    "lanes",
                                    "lanes:both_ways",
                                ]),
                            ));
                        }
                        let half = (remaining_lanes + 1) / 2; // usize division rounded up. (https://github.com/rust-lang/rust/issues/88581)
                        Self {
                            lanes: Infer::Direct(l),
                            forward: Infer::Direct(half),
                            backward: Infer::Direct(remaining_lanes.checked_sub(half).unwrap()),
                            both_ways,
                        }
                    }
                },
                (None, _, _) => {
                    // Without the "lanes" tag, assume one normal lane in each dir
                    // Adding any bus:lanes as needed.
                    const DEFAULT: usize = 1;
                    let tagged_bus_lanes: Option<usize> =
                        tags.get_parsed(&(LANES + "bus"), warnings);
                    let tagged_bus_forward = tags
                        .get_parsed(&(LANES + "bus" + "forward"), warnings)
                        .or_else(|| tagged_bus_lanes.map(|l| l / 2))
                        .unwrap_or(0);
                    let tagged_bus_backward = tags
                        .get_parsed(&(LANES + "bus" + "backward"), warnings)
                        .or_else(|| tagged_bus_lanes.map(|l| l / 2))
                        .unwrap_or(0);
                    let forward = match tagged_forward {
                        Some(f) => Infer::Direct(f),
                        None => Infer::Calculated(tagged_bus_forward + DEFAULT),
                    };
                    let backward = match tagged_backward {
                        Some(b) => Infer::Direct(b),
                        None => Infer::Default(tagged_bus_backward + DEFAULT),
                    };
                    let lanes = Infer::Default(
                        tagged_forward.unwrap_or(DEFAULT)
                            + tagged_backward.unwrap_or(DEFAULT)
                            + tagged_bus_lanes.unwrap_or_else(|| {
                                tagged_bus_forward.checked_add(tagged_bus_backward).unwrap()
                            })
                            + both_way_lanes,
                    );
                    Self {
                        lanes,
                        forward,
                        backward,
                        both_ways,
                    }
                },
            }
        };

        assert!(counts.forward.some().unwrap_or(0) >= pre_forward);
        assert!(counts.backward.some().unwrap_or(0) >= pre_backward);

        counts
    }

    /// Look at tags that use pipe separated access values to determine lane counts
    fn get_counts_by_access(tags: &Tags, warnings: &mut RoadWarnings) -> Self {
        const LANES: [TagKey; 2] = [TagKey::from("bus:lanes"), TagKey::from("psv:lanes")];
        const FORWARD: [TagKey; 2] = [
            TagKey::from("bus:lanes:forward"),
            TagKey::from("psv:lanes:forward"),
        ];
        const BACKWARD: [TagKey; 2] = [
            TagKey::from("bus:lanes:backward"),
            TagKey::from("psv:lanes:backward"),
        ];
        Self {
            lanes: Counts::get_lanes_by_access(tags, LANES.as_slice(), warnings)
                .map_or(Infer::None, Infer::Calculated),
            forward: Counts::get_lanes_by_access(tags, FORWARD.as_slice(), warnings)
                .map_or(Infer::None, Infer::Calculated),
            backward: Counts::get_lanes_by_access(tags, BACKWARD.as_slice(), warnings)
                .map_or(Infer::None, Infer::Calculated),
            both_ways: Infer::None,
        }
    }

    /// Look at tags that use pipe separated access values to determine lane count
    fn get_lanes_by_access(
        tags: &Tags,
        keys: &[TagKey],
        warnings: &mut RoadWarnings,
    ) -> Option<usize> {
        let lanes: Vec<usize> = keys
            .iter()
            .filter_map(|k| tags.get(k).map(|v| v.split('|').count()))
            .collect();
        if lanes.windows(2).any(|w| {
            let w: [usize; 2] = w.try_into().unwrap();
            w[0] != w[1]
        }) {
            warnings.push(TagsToLanesMsg::ambiguous(
                "different lane counts",
                tags.subset(keys),
            ));
        }
        lanes.first().copied()
    }
}

const CENTRE_TURN_LANE: TagKey = TagKey::from("centre_turn_lane");

struct CentreTurnLane(Option<bool>);

impl CentreTurnLane {
    /// Parses and validates the `centre_turn_lane` tag and emits a deprecation warning.
    /// See <https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane>.
    fn new(tags: &Tags, _oneway: Oneway, locale: &Locale, warnings: &mut RoadWarnings) -> Self {
        Self(match tags.get(CENTRE_TURN_LANE) {
            None => None,
            Some("yes") => {
                warnings.push(TagsToLanesMsg::deprecated(
                    tags.subset(&[CENTRE_TURN_LANE]),
                    Tags::from_str_pairs(&[
                        ["lanes:both_ways", "1"],
                        [
                            "turn:lanes:both_ways",
                            match locale.driving_side.opposite() {
                                DrivingSide::Left => "left",
                                DrivingSide::Right => "right",
                            },
                        ],
                    ])
                    .ok(),
                ));
                Some(true)
            },
            Some("no") => {
                warnings.push(TagsToLanesMsg::deprecated(
                    tags.subset(&[CENTRE_TURN_LANE]),
                    Some(Tags::from_str_pair(["lanes:both_ways", "0"])),
                ));
                Some(false)
            },
            Some(_) => {
                warnings.push(TagsToLanesMsg::deprecated(
                    tags.subset(&[CENTRE_TURN_LANE]),
                    Tags::from_str_pairs(&[
                        ["lanes:both_ways", "*"],
                        ["turn:lanes:both_ways", "*"],
                    ])
                    .ok(),
                ));
                // TODO what's the right warning for bad tag values?
                warnings.push(TagsToLanesMsg::unsupported_tags(
                    tags.subset(&[CENTRE_TURN_LANE]),
                ));
                None
            },
        })
    }
}
