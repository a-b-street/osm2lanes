use crate::locale::Locale;
use crate::tag::{TagKey, Tags};
use crate::transform::{RoadWarnings, TagsToLanesMsg};

/// Unsupported
///
/// # Errors
///
/// Oneway reversible
pub fn unsupported(
    tags: &Tags,
    _locale: &Locale,
    warnings: &mut RoadWarnings,
) -> Result<(), TagsToLanesMsg> {
    // https://wiki.openstreetmap.org/wiki/Key:access#Transport_mode_restrictions
    const ACCESS_KEYS: [&str; 43] = [
        "access",
        "dog",
        "ski",
        "inline_skates",
        "horse",
        "vehicle",
        "bicycle",
        "electric_bicycle",
        "carriage",
        "hand_cart",
        "quadracycle",
        "trailer",
        "caravan",
        "motor_vehicle",
        "motorcycle",
        "moped",
        "mofa",
        "motorcar",
        "motorhome",
        "tourist_bus",
        "coach",
        "goods",
        "hgv",
        "hgv_articulated",
        "bdouble",
        "agricultural",
        "golf_cart",
        "atv",
        "snowmobile",
        "psv",
        "bus",
        "taxi",
        "minibus",
        "share_taxi",
        "hov",
        "car_sharing",
        "emergency",
        "hazmat",
        "disabled",
        "roadtrain",
        "hgv_caravan",
        "lhv",
        "tank",
    ];
    if ACCESS_KEYS
        .iter()
        .any(|k| tags.get(TagKey::from(k)).is_some())
    {
        warnings.push(TagsToLanesMsg::unimplemented(
            "access",
            // TODO, TagTree should support subset
            tags.subset(&ACCESS_KEYS),
        ));
    }

    if let Some(v @ ("reversible" | "-1")) = tags.get("oneway") {
        // TODO reversible roads should be handled differently
        return Err(TagsToLanesMsg::unimplemented_tag("oneway", v));
    }

    Ok(())
}
