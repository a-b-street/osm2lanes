use wasm_bindgen::prelude::*;

use crate::tags::Tags;
use crate::{tags_to_lanes, Locale, TagsToLanesConfig};

#[wasm_bindgen]
pub fn tags_to_lanes_tmp(tags: &JsValue) -> JsValue {
    utils::set_panic_hook();
    let tags: Tags = tags.into_serde().unwrap();
    let locale = Locale::builder().build();
    let lanes = tags_to_lanes(&tags, &locale, &TagsToLanesConfig::default());
    JsValue::from_serde(&lanes).unwrap()
}

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
