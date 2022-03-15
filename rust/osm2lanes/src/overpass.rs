use serde::Deserialize;

use crate::tag::Tags;
use crate::{DrivingSide, Locale};

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
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "way")]
    Way,
    #[serde(rename = "area")]
    Area,
}

type ElementId = u64;

pub async fn get_tags(id: ElementId) -> Result<Tags, reqwest::Error> {
    let mut resp = reqwest::Client::new()
        .get(format!(
            "https://overpass-api.de/api/interpreter?data=[out:json][timeout:2];way(id:{});out tags;",
            id
        ))
        .send()
        .await?
        .json::<OverpassResult>()
        .await?;
    log::debug!("{:#?}", resp);
    assert_eq!(resp.elements.len(), 1);
    let element = resp.elements.pop().unwrap();
    assert_eq!(element.r#type, ElementType::Way);
    assert_eq!(element.id, id);
    Ok(element.tags)
}

pub async fn get_iso3166_2(id: ElementId) -> Result<String, reqwest::Error> {
    let mut resp = reqwest::Client::new()
        .get(format!(
            r#"https://overpass-api.de/api/interpreter?data=[out:json][timeout:2];way(id:{});>;is_in;area._["ISO3166-2"];out tags;"#,
            id
        ))
        .send()
        .await?
        .json::<OverpassResult>()
        .await?;
    log::debug!("{:#?}", resp);
    assert_eq!(resp.elements.len(), 1);
    let element = resp.elements.pop().unwrap();
    Ok(element.tags.get("ISO3166-2").unwrap().to_owned())
}

pub async fn get_way(id: ElementId) -> Result<(Tags, Locale), reqwest::Error> {
    let resp = reqwest::Client::new()
        .get(format!(
            r#"https://overpass-api.de/api/interpreter?data=[out:json][timeout:25];
            way(id:{id});
            out tags;
            way(id:{id});
            >;
            is_in->.enclosing;
            (
              area.enclosing["ISO3166-2"];
              area.enclosing["ISO3166-1"];
              area.enclosing["driving_side"];
            );
            out tags;"#
        ))
        .send()
        .await?
        .json::<OverpassResult>()
        .await?;
    log::debug!("{:#?}", resp);

    let iso3166_2 = resp
        .elements
        .iter()
        .find_map(|element| element.tags.get("ISO3166-2"));
    let iso3166_1 = resp
        .elements
        .iter()
        .find_map(|element| element.tags.get("ISO3166-1"));
    let driving_side = resp
        .elements
        .iter()
        .find_map(|element| element.tags.get("driving_side"));
    let locale = Locale::builder()
        .driving_side(driving_side.map_or(DrivingSide::Right, |d| d.parse().unwrap()))
        .iso_3166_option(iso3166_2.or(iso3166_1))
        .build();

    let way_element = {
        let mut elements = resp.elements;
        elements.truncate(1);
        elements.pop().unwrap()
    };
    assert_eq!(way_element.r#type, ElementType::Way);
    assert_eq!(way_element.id, id);

    Ok((way_element.tags, locale))
}
