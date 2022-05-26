use crate::locale::Locale;
use crate::road::{Designated, Direction};
use crate::tag::{Access, Tags, HIGHWAY};
use crate::transform::tags_to_lanes::{RoadBuilder, TagsToLanesMsg};
use crate::transform::{Infer, RoadWarnings};

#[allow(clippy::unnecessary_wraps, clippy::restriction)]
pub(in crate::transform::tags_to_lanes) fn non_motorized(
    tags: &Tags,
    _locale: &Locale,
    road: &mut RoadBuilder,
    warnings: &mut RoadWarnings,
) -> Result<(), TagsToLanesMsg> {
    if road.highway.is_supported_non_motorized() {
        log::trace!("non-motorized");
    } else {
        log::trace!("motorized");
        return Ok(());
    }
    // Easy special cases first.
    if let Some(v @ ("steps" | "path")) = tags.get(&HIGHWAY) {
        // TODO: how to avoid making this assumption?
        assert_eq!(road.len(), 1);
        let lane = road.forward_outside_mut().unwrap();
        lane.designated.set(Infer::Direct(Designated::Foot))?;
        lane.direction.set(Infer::Direct(Direction::Both))?;
        lane.access.foot.set(Infer::Direct(Access::Designated))?;
        lane.access.motor.set(Infer::Direct(Access::No))?;
        if v == "steps" {
            warnings.push(TagsToLanesMsg::unimplemented(
                "steps becomes sidewalk",
                tags.subset(&[HIGHWAY]),
            ));
        }
    }

    Ok(())
}
