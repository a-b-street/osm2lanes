use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::tag::{Tags, TagsWrite};
use crate::{tags_to_lanes, DrivingSide, Locale, TagsToLanesConfig};

#[derive(Serialize, Deserialize)]
pub struct Input {
    key_values: HashMap<String, String>,
    drive_on_right: bool,
}

#[wasm_bindgen]
pub fn js_tags_to_lanes(val: &JsValue) -> JsValue {
    console_error_panic_hook::set_once();

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
