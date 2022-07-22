mod utils;

use std::collections::HashMap;

use osm2lanes::locale::{DrivingSide, Locale};
use osm2lanes::transform::{tags_to_lanes, TagsToLanesConfig, lanes_to_tags, LanesToTagsConfig};
use osm2lanes::overpass::get_way;
use osm2lanes::road::Road;
use osm_tags::Tags;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// TODO Rename and document things.

#[derive(Serialize, Deserialize)]
pub struct Input {
    key_values: HashMap<String, String>,
    drive_on_right: bool,
}

#[wasm_bindgen]
pub fn js_tags_to_lanes(val: &JsValue) -> JsValue {
    utils::set_panic_hook();

    let input: Input = val.into_serde().unwrap();

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
        tags.checked_insert(key, value).unwrap();
    }
    let lanes = tags_to_lanes(&tags, &locale, &config).unwrap();
    JsValue::from_serde(&lanes).unwrap()
}

#[wasm_bindgen]
pub async fn js_way_to_lanes(osm_way_id: u64) -> JsValue {
    utils::set_panic_hook();

    // TODO Fix get_way's API
    let (tags, _geom, locale) = get_way(&osm_way_id).await.unwrap();
    let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
    // Also return the locale
    JsValue::from_serde(&(lanes, locale)).unwrap()
}

#[wasm_bindgen]
pub fn js_lanes_to_tags(road: &JsValue, locale: &JsValue) -> String {
    utils::set_panic_hook();

    let road: Road = road.into_serde().unwrap();
    let locale: Locale = locale.into_serde().unwrap();
    let tags = lanes_to_tags(&road, &locale, &LanesToTagsConfig::new(false)).unwrap();
    tags.to_string()
}
