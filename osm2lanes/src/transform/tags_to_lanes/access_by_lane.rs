use std::borrow::Borrow;
use std::hash::Hash;

use osm_tags::{TagKey, Tags};

use super::road::RoadBuilder;
use crate::locale::Locale;
use crate::transform::tags_to_lanes::TagsToLanesMsg;
use crate::transform::RoadWarnings;

/// <https://wiki.openstreetmap.org/wiki/Key:access#Lane_dependent_restrictions>

#[derive(Debug)]
pub(in crate::transform::tags_to_lanes) enum Access {
    None,
    No,
    Yes,
    Designated,
}

impl std::str::FromStr for Access {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Self::None),
            "no" => Ok(Self::No),
            "yes" => Ok(Self::Yes),
            "designated" => Ok(Self::Designated),
            _ => Err(s.to_owned()),
        }
    }
}

impl Access {
    pub(in crate::transform::tags_to_lanes) fn split(lanes: &str) -> Result<Vec<Self>, String> {
        lanes.split('|').map(str::parse).collect()
    }
}

#[derive(Debug)]
pub(in crate::transform::tags_to_lanes) enum LaneDependentAccess {
    LeftToRight(Vec<Access>),
    Forward(Vec<Access>),
    Backward(Vec<Access>),
    ForwardBackward {
        forward: Vec<Access>,
        backward: Vec<Access>,
    },
}

/// Get value from tags given a key
pub(in crate::transform::tags_to_lanes) fn get_access<Q, O>(
    tags: &Tags,
    k: &Q,
) -> Result<Option<Vec<Access>>, TagsToLanesMsg>
where
    TagKey: Borrow<Q>,
    Q: Ord + Hash + Eq + ?Sized + ToOwned<Owned = O>,
    O: Into<TagKey>,
{
    tags.get(k)
        .map(|a| {
            Access::split(a).map_err(|a| {
                TagsToLanesMsg::unsupported(&format!("lanes access {}", a), tags.subset([k]))
            })
        })
        .transpose()
}

impl LaneDependentAccess {
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn from_tags(
        key: &TagKey,
        tags: &Tags,
        _locale: &Locale,
        road: &RoadBuilder,
        warnings: &mut RoadWarnings,
    ) -> Result<Option<Self>, TagsToLanesMsg> {
        const LANES: TagKey = TagKey::from_static("lanes");
        // Unstable: const evaluation https://github.com/rust-lang/rust/issues/90080
        let lanes_forward = LANES + "forward";
        let lanes_backward = LANES + "backward";
        let key_forward = key + "forward";
        let key_backward = key + "backward";
        Ok(
            match (
                get_access(tags, key)?,
                (
                    get_access(tags, &key_forward)?,
                    get_access(tags, &key_backward)?,
                ),
            ) {
                (Some(lanes), (None, None)) => {
                    if lanes.len() != road.len() {
                        return Err(TagsToLanesMsg::unsupported(
                            "lane count mismatch",
                            tags.subset([key, &LANES, &(LANES + "forward"), &(LANES + "backward")]),
                        ));
                    }
                    Some(Self::LeftToRight(lanes))
                },
                (None, (Some(forward), None)) => Some(Self::Forward(forward)),
                (None, (None, Some(backward))) => Some(Self::Backward(backward)),
                (total, (Some(forward), Some(backward))) => {
                    if forward.len().checked_add(backward.len()) != Some(road.len()) {
                        return Err(TagsToLanesMsg::unsupported(
                            "lane count mismatch",
                            tags.subset([
                                &key_forward,
                                &key_backward,
                                &LANES,
                                &lanes_forward,
                                &lanes_backward,
                            ]),
                        ));
                    }
                    if total.is_some() {
                        warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset([
                            key,
                            &key_forward,
                            &key_backward,
                        ])));
                    }
                    Some(Self::ForwardBackward { forward, backward })
                },
                (None, (None, None)) => None,
                (Some(_), (Some(_), None) | (None, Some(_))) => {
                    return Err(TagsToLanesMsg::ambiguous_tags(tags.subset([
                        key,
                        &key_forward,
                        &key_backward,
                    ])))
                },
            },
        )
    }
}
