use serde::Deserialize;

use crate::locale::{DrivingSide, Locale};
use crate::tag::Tags;

#[derive(Debug, Clone, Deserialize)]
struct OverpassResult {
    // version: f32,
    // osm3s: Osm3s
    elements: Vec<Element>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Element {
    r#type: ElementType,
    id: ElementId,
    tags: Tags,
    geometry: Option<Vec<LatLon>>,
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

/// Get Tags from Overpass
///
/// # Errors
///
/// May occur when processing a request.
///
/// # Panics
///
/// Unexpected data from overpass and/or openstreetmap.
///
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

/// Get Way from Overpass
///
/// # Errors
///
/// May occur when processing a request.
///
/// # Panics
///
/// Unexpected data from overpass and/or openstreetmap.
///
pub async fn get_way(id: &ElementId) -> Result<(Tags, Locale), reqwest::Error> {
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
    assert_eq!(&way_element.id, id);

    Ok((way_element.tags, locale))
}

/// Get Tags and Geometries from Overpass.
/// Given a longitude and latitude, find a way within 10m.
/// If there is more than one way within 10m, returns one at random...
///
/// # Errors
///
/// May occur when processing a request.
///
/// # Panics
///
/// Unexpected data from overpass and/or openstreetmap.
///
pub async fn get_nearby(
    lng_lat: (f64, f64),
) -> Result<(Tags, Locale, Vec<LatLon>), reqwest::Error> {
    const RADIUS: f64 = 10.0_f64;
    let mut resp = reqwest::Client::new()
        .get(format!(
            r#"https://overpass-api.de/api/interpreter?data=[out:json][timeout:25];
            way
            (around:{RADIUS},{},{})
            ["highway"];
            out tags geom;"#,
            lng_lat.0, lng_lat.1
        ))
        .send()
        .await?
        .json::<OverpassResult>()
        .await?;
    log::debug!("{:#?}", resp);
    if resp.elements.len() > 1 {
        log::warn!("more than one nearby way found, returning one at random");
    }
    resp.elements.truncate(1);
    let element = resp.elements.pop().unwrap();
    assert_eq!(element.r#type, ElementType::Way);
    // TODO: single overpass query
    let (tags, locale) = get_way(&element.id).await?;
    Ok((
        tags,
        locale,
        element
            .geometry
            .expect("overpass response missing geometry"),
    ))
}
