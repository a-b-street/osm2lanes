use super::*;

pub fn parking(
    tags: &Tags,
    locale: &Locale,
    _oneway: bool,
    forward_side: &mut Vec<Lane>,
    backward_side: &mut Vec<Lane>,
) {
    let has_parking = vec!["parallel", "diagonal", "perpendicular"];
    let parking_lane_fwd = tags.is_any("parking:lane:right", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    let parking_lane_back = tags.is_any("parking:lane:left", &has_parking)
        || tags.is_any("parking:lane:both", &has_parking);
    if parking_lane_fwd {
        forward_side.push(Lane::parking(LaneDirection::Forward, locale));
    }
    if parking_lane_back {
        backward_side.push(Lane::parking(LaneDirection::Backward, locale));
    }
}
