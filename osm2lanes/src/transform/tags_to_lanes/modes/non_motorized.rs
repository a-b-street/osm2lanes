use osm_tag_schemes::keys::HIGHWAY;
use osm_tag_schemes::Access;
use osm_tags::Tags;

use crate::locale::Locale;
use crate::road::{AccessAndDirection, Designated, Direction};
use crate::transform::tags_to_lanes::{RoadBuilder, TagsToLanesMsg};
use crate::transform::{Infer, RoadWarnings};

#[allow(clippy::unnecessary_wraps, clippy::restriction)]
pub(in crate::transform::tags_to_lanes) fn non_motorized(
    tags: &Tags,
    _locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), TagsToLanesMsg> {
    // Easy special cases first.
    if let Some(v @ ("steps" | "path")) = tags.get(&HIGHWAY) {
        // TODO: how to avoid making this assumption?
        assert_eq!(road.len(), 1);
        let lane = road.forward_outside_mut().unwrap();
        lane.designated.set(Infer::Direct(Designated::Foot))?;
        lane.direction.set(Infer::Direct(Direction::Both))?;
        lane.access.foot.set(Infer::Direct(AccessAndDirection {
            access: Access::Designated,
            direction: None,
        }))?;
        lane.access.motor.set(Infer::Direct(AccessAndDirection {
            access: Access::No,
            direction: None,
        }))?;
        if v == "steps" {
            warnings.push(TagsToLanesMsg::unimplemented(
                "steps becomes sidewalk",
                tags.subset(&[HIGHWAY]),
            ));
        }
    }

    Ok(())
}
