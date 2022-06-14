use osm_tags::Tags;

use crate::locale::Locale;
use crate::road::{Designated, Direction};
use crate::transform::tags_to_lanes::{Infer, LaneBuilder, LaneType, RoadBuilder};
use crate::transform::RoadError;

impl LaneBuilder {
    fn parking_forward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Parking),
            direction: Infer::Direct(Direction::Forward),
            designated: Infer::Direct(Designated::Motor),
            ..Default::default()
        }
    }
    fn parking_backward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Parking),
            direction: Infer::Direct(Direction::Backward),
            designated: Infer::Direct(Designated::Motor),
            ..Default::default()
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(in crate::transform::tags_to_lanes) fn parking(
    tags: &Tags,
    locale: &Locale,
    road: &mut RoadBuilder,
) -> Result<(), RoadError> {
    let has_parking = vec!["parallel", "diagonal", "perpendicular"];
    let parking_lane_fwd = tags.is_any("parking:lane:right", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    let parking_lane_back = tags.is_any("parking:lane:left", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    if parking_lane_fwd {
        road.push_forward_outside(LaneBuilder::parking_forward(locale));
    }
    if parking_lane_back {
        road.push_backward_outside(LaneBuilder::parking_backward(locale));
    }
    Ok(())
}
