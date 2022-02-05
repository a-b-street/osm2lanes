use osm2lanes::tags::Tags;
use osm2lanes::{tags_to_lanes, Locale, TagsToLanesConfig};
use wasm_bindgen::prelude::*;

mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn tags_to_lanes_tmp(tags: &JsValue) -> JsValue {
    utils::set_panic_hook();
    let tags: Tags = tags.into_serde().unwrap();
    let locale = Locale::builder().build();
    let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
    JsValue::from_serde(&lanes).unwrap()
}
