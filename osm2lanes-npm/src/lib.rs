mod utils;

use std::collections::HashMap;

use osm2lanes::locale::{DrivingSide, Locale};
use osm2lanes::transform::{tags_to_lanes, TagsToLanesConfig};
use osm_tags::Tags;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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
