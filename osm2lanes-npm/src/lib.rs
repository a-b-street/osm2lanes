use std::collections::HashMap;

use osm2lanes::locale::{DrivingSide, Locale};
use osm2lanes::overpass::get_way;
use osm2lanes::road::Road;
use osm2lanes::transform::{lanes_to_tags, tags_to_lanes, LanesToTagsConfig, TagsToLanesConfig};
use osm_tags::Tags;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// TODO Rename and document things.

#[derive(Serialize, Deserialize)]
pub struct Input {
    key_values: HashMap<String, String>,
    drive_on_right: bool,
}

#[wasm_bindgen]
pub fn js_tags_to_lanes(val: &JsValue) -> Result<JsValue, JsValue> {
    // Panics shouldn't happen, but if they do, console.log them.
    console_error_panic_hook::set_once();

    let input: Input = val.into_serde().map_err(err_to_string)?;

    let mut config = TagsToLanesConfig::default();
    config.error_on_warnings = false;
    config.include_separators = true;

    let locale = Locale::builder()
        .driving_side(if input.drive_on_right {
            DrivingSide::Right
        } else {
            DrivingSide::Left
        })
        .build();

    let mut tags = Tags::default();
    for (key, value) in input.key_values {
        tags.checked_insert(key, value).map_err(err_to_string)?;
    }
    let lanes = tags_to_lanes(&tags, &locale, &config).map_err(err_to_string)?;
    JsValue::from_serde(&lanes).map_err(err_to_string)
}

#[wasm_bindgen]
pub async fn js_way_to_lanes(osm_way_id: u64) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let (tags, _geom, locale) = get_way(osm_way_id).await.map_err(err_to_string)?;
    let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
    // Also return the tags and locale
    JsValue::from_serde(&(lanes, locale, tags)).map_err(err_to_string)
}

#[wasm_bindgen]
pub fn js_lanes_to_tags(road: &JsValue, locale: &JsValue) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let road: Road = road.into_serde().map_err(err_to_string)?;
    let locale: Locale = locale.into_serde().map_err(err_to_string)?;
    let tags =
        lanes_to_tags(&road, &locale, &LanesToTagsConfig::new(false)).map_err(err_to_string)?;
    JsValue::from_serde(&tags).map_err(err_to_string)
}

fn err_to_string<T: std::fmt::Display>(err: T) -> JsValue {
    JsValue::from_str(&err.to_string())
}
