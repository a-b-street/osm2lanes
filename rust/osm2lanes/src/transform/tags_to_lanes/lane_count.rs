use crate::locale::{DrivingSide, Locale};
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
    /// See https://wiki.openstreetmap.org/wiki/Key:lanes.
    ///
    /// Validates `lanes[:{forward,both_ways,backward}]=*` and `centre_turn_lane=yes`.
    pub(in crate::transform::tags_to_lanes) fn new(
        tags: &Tags,
        road: &RoadBuilder,
        _locale: &Locale,
        warnings: &mut RoadWarnings,
    ) -> Self {
        // TODO, crosscheck access against tagged
        let access = Self::get_counts_by_access(tags, warnings);

        // The tags for this schema (which we will validate).
        let tagged_lanes: Option<usize> = tags.get_parsed(&LANES, warnings).or(access.lanes.some());
        let tagged_forward: Option<usize> = tags
            .get_parsed(&(LANES + "forward"), warnings)
            .or(access.forward.some());
        let tagged_backward: Option<usize> = tags
            .get_parsed(&(LANES + "backward"), warnings)
            .or(access.backward.some());
        let tagged_both_ways: Option<usize> = tags
            .get_parsed(&(LANES + "both_ways"), warnings)
            .or(access.both_ways.some());

        let centre_turn_lane = CentreTurnLane::new(tags, road.oneway, _locale, warnings);

        // Calculate the bothways lanes.
        let both_ways = match (tagged_both_ways, centre_turn_lane.0.some()) {
            (Some(bw), None) => Infer::Direct(bw),
            (None, Some(true)) => Infer::Calculated(1),
            (None, Some(false)) => Infer::Calculated(0),
            (Some(bw), Some(ctl)) => {
                // TODO what if the values conflict but are not Direct? Might not ever happen.
                if (bw > 0 && !ctl) || (bw == 0 && ctl) {
                    warnings.push(TagsToLanesMsg::ambiguous_tags(
                        tags.subset(&[LANES + "both_ways", CENTRE_TURN_LANE]),
                    ));
                }
                Infer::Direct(bw)
            },
            (None, None) => Infer::Default(0),
        };
        let both_way_lanes = both_ways.some().unwrap_or(0);

        if road.oneway.into() {
            // Ignore lanes:{both_ways,backward}=
            if both_ways.some().is_some() || tagged_backward.is_some() {
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
                        tags.subset(&["oneway", "lanes", "lanes:forward"]),
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
                    || tagged_lanes.map_or(Infer::Default(assumed_forward), Infer::Direct),
                    Infer::Direct,
                ),
                backward: tagged_backward.map_or(Infer::Default(0), Infer::Direct),
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
                    backward: Infer::Calculated(l - f - both_way_lanes),
                    both_ways,
                },
                (Some(l), None, Some(b)) => Self {
                    lanes: Infer::Direct(l),
                    forward: Infer::Calculated(l - b - both_way_lanes),
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
                    if l % 2 == 0 && centre_turn_lane.0.some().unwrap_or(false) {
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
                        let remaining_lanes = l - both_way_lanes;
                        if remaining_lanes % 2 != 0 {
                            warnings.push(TagsToLanesMsg::ambiguous("Total lane count cannot be evenly divided between the forward and backward",
                                tags.subset(&[
                                    "lanes",
                                    "lanes:both_ways",
                                ]),
                            ));
                        }
                        let half = (remaining_lanes + 1) / 2; // usize division rounded up.
                        Self {
                            lanes: Infer::Direct(l),
                            forward: Infer::Direct(half),
                            backward: Infer::Direct(remaining_lanes - half),
                            both_ways,
                        }
                    }
                },
                (None, _, _) => {
                    // Without the "lanes" tag, assume one normal lane in each dir
                    let f = tagged_forward.unwrap_or(1);
                    let b = tagged_backward.unwrap_or(1);
                    let forward = match tagged_forward.is_some() {
                        true => Infer::Direct(f),
                        false => Infer::Default(f),
                    };
                    let backward = match tagged_backward.is_some() {
                        true => Infer::Direct(b),
                        false => Infer::Default(b),
                    };
                    let lanes = Infer::Default(f + b + both_way_lanes);
                    Self {
                        lanes,
                        forward,
                        backward,
                        both_ways,
                    }
                },
            }
        }
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
            .filter_map(|k| tags.get(k).and_then(|v| Some(v.split('|').count())))
            .collect();
        if lanes.windows(2).any(|w| w[0] != w[1]) {
            warnings.push(TagsToLanesMsg::ambiguous(
                "different lane counts",
                tags.subset(keys),
            ))
        }
        lanes.first().copied()
    }
}

const CENTRE_TURN_LANE: TagKey = TagKey::from("centre_turn_lane");

struct CentreTurnLane(Infer<bool>);

impl CentreTurnLane {
    /// Parses and validates the `centre_turn_lane` tag and emits a deprecation warning.
    /// See https://wiki.openstreetmap.org/wiki/Key:centre_turn_lane.
    fn new(tags: &Tags, _oneway: Oneway, locale: &Locale, warnings: &mut RoadWarnings) -> Self {
        Self(match tags.get(CENTRE_TURN_LANE) {
            None => Infer::Default(false),
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
                Infer::Direct(true)
            },
            Some("no") => {
                warnings.push(TagsToLanesMsg::deprecated(
                    tags.subset(&[CENTRE_TURN_LANE]),
                    Some(Tags::from_str_pair(["lanes:both_ways", "0"])),
                ));
                Infer::Direct(false)
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
                Infer::Default(false)
            },
        })
    }
}
