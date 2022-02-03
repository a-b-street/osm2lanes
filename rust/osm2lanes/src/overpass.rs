use serde::Deserialize;

use crate::tags::Tags;

#[derive(Debug, Clone, Deserialize)]
struct OverpassResult {
    // version: f32,
    // osm3s: Osm3s
    elements: Vec<Element>,
}

#[derive(Debug, Clone, Deserialize)]
struct Element {
    r#type: ElementType,
    id: ElementId,
    tags: Tags,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
enum ElementType {
    #[serde(rename = "way")]
    Way,
}

type ElementId = u64;

pub fn get_way(id: ElementId) -> Tags {
    let mut resp = reqwest::blocking::get(format!(
        "https://overpass-api.de/api/interpreter?data=[out:json][timeout:2];way(id:{});out tags;",
        id
    ))
    .unwrap()
    .json::<OverpassResult>()
    .unwrap();
    log::debug!("{:#?}", resp);
    assert_eq!(resp.elements.len(), 1);
    let element = resp.elements.pop().unwrap();
    assert_eq!(element.r#type, ElementType::Way);
    assert_eq!(element.id, id);
    element.tags
}
