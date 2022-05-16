use super::road::RoadBuilder;
use crate::locale::Locale;
use crate::tag::{TagKey, Tags};
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
pub(in crate::transform::tags_to_lanes) fn get_access<K>(
    tags: &Tags,
    k: K,
) -> Result<Option<Vec<Access>>, TagsToLanesMsg>
where
    K: AsRef<str> + Clone,
{
    tags.get(k.clone())
        .map(|a| {
            Access::split(a).map_err(|a| {
                TagsToLanesMsg::unsupported(&format!("lanes access {}", a), tags.subset(&[k]))
            })
        })
        .transpose()
}

impl LaneDependentAccess {
    // TODO: so much cloning!
    // Look at <https://github.com/a-b-street/osm2lanes/issues/78>
    #[allow(clippy::unnecessary_wraps)]
    pub fn from_tags<K>(
        key: K,
        tags: &Tags,
        _locale: &Locale,
        road: &RoadBuilder,
        warnings: &mut RoadWarnings,
    ) -> Result<Option<Self>, TagsToLanesMsg>
    where
        TagKey: From<K>,
    {
        const LANES: TagKey = TagKey::from_static("lanes");
        let key: TagKey = key.into();
        Ok(
            match (
                get_access(tags, key.clone())?,
                (
                    get_access(tags, key.clone() + "forward")?,
                    get_access(tags, key.clone() + "backward")?,
                ),
            ) {
                (Some(lanes), (None, None)) => {
                    if lanes.len() != road.len() {
                        return Err(TagsToLanesMsg::unsupported(
                            "lane count mismatch",
                            tags.subset(&[key, LANES, LANES + "forward", LANES + "backward"]),
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
                            tags.subset(&[
                                key.clone() + "forward",
                                key + "backward",
                                LANES,
                                LANES + "forward",
                                LANES + "backward",
                            ]),
                        ));
                    }
                    if total.is_some() {
                        warnings.push(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                            key.clone(),
                            key.clone() + "forward",
                            key + "backward",
                        ])));
                    }
                    Some(Self::ForwardBackward { forward, backward })
                },
                (None, (None, None)) => None,
                (Some(_), (Some(_), None) | (None, Some(_))) => {
                    return Err(TagsToLanesMsg::ambiguous_tags(tags.subset(&[
                        key.clone(),
                        key.clone() + "forward",
                        key + "backward",
                    ])))
                },
            },
        )
    }
}
