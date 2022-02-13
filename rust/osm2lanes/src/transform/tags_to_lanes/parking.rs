use super::*;

impl LaneBuilder {
    fn parking_forward() -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Parking),
            direction: Infer::Direct(LaneDirection::Forward),
            designated: Infer::Direct(LaneDesignated::Motor),
        }
    }
    fn parking_backward() -> Self {
        Self {
            r#type: Infer::Direct(LaneType::Parking),
            direction: Infer::Direct(LaneDirection::Backward),
            designated: Infer::Direct(LaneDesignated::Motor),
        }
    }
}

pub(super) fn parking(
    tags: &Tags,
    _locale: &Locale,
    _oneway: Oneway,
    forward_side: &mut Vec<LaneBuilder>,
    backward_side: &mut Vec<LaneBuilder>,
) -> ModeResult {
    let has_parking = vec!["parallel", "diagonal", "perpendicular"];
    let parking_lane_fwd = tags.is_any("parking:lane:right", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    let parking_lane_back = tags.is_any("parking:lane:left", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    if parking_lane_fwd {
        forward_side.push(LaneBuilder::parking_forward());
    }
    if parking_lane_back {
        backward_side.push(LaneBuilder::parking_backward());
    }
    Ok(())
}
