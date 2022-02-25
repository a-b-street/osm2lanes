use super::*;

impl LaneBuilder {
    fn parking_forward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Parking),
            direction: Infer::Direct(LaneDirection::Forward),
            designated: Infer::Direct(LaneDesignated::Motor),
            ..Default::default()
        }
    }
    fn parking_backward(_locale: &Locale) -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Parking),
            direction: Infer::Direct(LaneDirection::Backward),
            designated: Infer::Direct(LaneDesignated::Motor),
            ..Default::default()
        }
    }
}

pub(super) fn parking(tags: &Tags, locale: &Locale, road: &mut RoadBuilder) -> ModeResult {
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
