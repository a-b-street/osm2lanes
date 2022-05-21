use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::{LineString, Point};
use serde::Deserialize;

use crate::locale::{DrivingSide, Locale};
use crate::tag::Tags;

#[derive(Debug, Clone, Deserialize)]
struct OverpassResponse {
    // version: f32,
    // osm3s: Osm3s
    elements: Vec<Element>,
}

impl OverpassResponse {
    fn iso3166_2(&self) -> Option<&str> {
        self.elements
            .iter()
            .find_map(|element| element.tags.get("ISO3166-2"))
    }
    fn iso3166_1(&self) -> Option<&str> {
        self.elements
            .iter()
            .find_map(|element| element.tags.get("ISO3166-1"))
    }
    fn driving_side(&self) -> Option<&str> {
        self.elements
            .iter()
            .find_map(|element| element.tags.get("driving_side"))
    }
    fn locale(&self) -> Locale {
        Locale::builder()
            .driving_side(
                self.driving_side()
                    .map_or(DrivingSide::Right, |d| d.parse().unwrap()),
            )
            .iso_3166_option(self.iso3166_2().or_else(|| self.iso3166_1()))
            .build()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

// TODO: zero-cost deserialization from lat+lon into LineString.
// This currently initializes the memory twice.
// LineString deserialization expects x+y fields, alias needed.
fn convert(vec: &[LatLon]) -> LineString<f64> {
    vec.iter()
        .map(|lat_lon| [lat_lon.lat, lat_lon.lon])
        .collect()
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

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Empty,
    Malformed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reqwest(e) => e.fmt(f),
            Self::Empty => write!(f, "overpass response empty"),
            Self::Malformed => write!(f, "overpass response malformed"),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

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
pub async fn get_tags(id: &ElementId) -> Result<Tags, Error> {
    let mut resp = reqwest::Client::new()
        .get(format!(
            "https://overpass-api.de/api/interpreter?data=[out:json][timeout:2];way(id:{});out tags;",
            id
        ))
        .send()
        .await?
        .json::<OverpassResponse>()
        .await?;
    log::debug!("{:#?}", resp);
    let way_element = resp.elements.pop().ok_or(Error::Empty)?;
    if !resp.elements.is_empty() || way_element.r#type != ElementType::Way || &way_element.id != id
    {
        return Err(Error::Malformed);
    }
    Ok(way_element.tags)
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
pub async fn get_way(id: &ElementId) -> Result<(Tags, LineString<f64>, Locale), Error> {
    let resp = reqwest::Client::new()
        .get(format!(
            r#"https://overpass-api.de/api/interpreter?data=[out:json][timeout:25];
            way(id:{id});
            out tags geom;
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
        .json::<OverpassResponse>()
        .await?;
    log::debug!("{:#?}", resp);

    let locale = resp.locale();
    let way_element = {
        let mut elements = resp.elements;
        elements.truncate(1);
        elements.pop().ok_or(Error::Empty)?
    };

    if way_element.r#type != ElementType::Way || &way_element.id != id {
        return Err(Error::Malformed);
    }
    Ok((
        way_element.tags,
        convert(&way_element.geometry.ok_or(Error::Malformed)?),
        locale,
    ))
}

/// Get Tags and Geometries from Overpass.
/// Given a longitude and latitude, find the nearest way within 100m by euclidean distance.
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
    point: Point<f64>,
) -> Result<(ElementId, Tags, LineString<f64>, Locale), Error> {
    const RADIUS: f64 = 100.0_f64;
    let lat = point.x();
    let lon = point.y();
    let resp = reqwest::Client::new()
        .get(format!(
            r#"https://overpass-api.de/api/interpreter?data=[out:json][timeout:25];
            way
                (around:{RADIUS},{lat},{lon})
                ["highway"];
            out tags geom;
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
        .json::<OverpassResponse>()
        .await?;
    log::debug!("{:#?}", resp);

    let locale = resp.locale();
    let (way_element, geometry, _distance) = resp
        .elements
        .into_iter()
        .filter_map(|element| {
            let geometry = element.geometry.as_ref().map(|g| convert(g));
            geometry.map(|geometry| {
                let distance = point.euclidean_distance(&geometry);
                (element, geometry, distance)
            })
        })
        .min_by(|(_, _, left_distance), (_, _, right_distance)| {
            left_distance.partial_cmp(right_distance).unwrap()
        })
        .ok_or(Error::Empty)?;

    if way_element.r#type != ElementType::Way {
        return Err(Error::Malformed);
    }

    Ok((way_element.id, way_element.tags, geometry, locale))
}

#[cfg(test)]
mod tests {
    const RESPONSE: &str = r#"
    {
        "version": 0.6,
        "generator": "Overpass API 0.7.57 93a4d346",
        "osm3s": {
          "timestamp_osm_base": "2022-05-17T02:05:52Z",
          "timestamp_areas_base": "2022-05-17T01:08:26Z",
          "copyright": "The data included in this document is from www.openstreetmap.org. The data is made available under ODbL."
        },
        "elements": [
      
      {
        "type": "way",
        "id": 62176050,
        "bounds": {
          "minlat": -25.2002597,
          "minlon": 119.3297027,
          "maxlat": -24.7125125,
          "maxlon": 119.6076495
        },
        "geometry": [
          { "lat": -25.2002597, "lon": 119.3345558 },
          { "lat": -25.1981968, "lon": 119.3332791 },
          { "lat": -25.1940244, "lon": 119.3307166 },
          { "lat": -24.7125125, "lon": 119.6076495 }
        ],
        "tags": {
          "highway": "trunk",
          "maxspeed": "110",
          "name": "Great Northern Highway",
          "network": "NH",
          "ref": "95",
          "source:geometry": "Esri World Imagery",
          "surface": "asphalt"
        }
      },
      {
        "type": "area",
        "id": 3600080500,
        "tags": {
          "ISO3166-1": "AU",
          "ISO3166-1:alpha2": "AU",
          "ISO3166-1:alpha3": "AUS",
          "ISO3166-1:numeric": "036",
          "admin_level": "2",
          "boundary": "administrative",
          "contact:website": "http://australia.gov.au",
          "default_language": "en",
          "driving_side": "left",
          "flag": "http://upload.wikimedia.org/wikipedia/commons/b/b9/Flag_of_Australia.svg",
          "int_name": "Australia",
          "name": "Australia",
          "type": "boundary",
          "website:tourism": "http://www.australia.com",
          "wikidata": "Q408",
          "wikipedia": "en:Australia"
        }
      },
      {
        "type": "area",
        "id": 3602316598,
        "tags": {
          "ISO3166-2": "AU-WA",
          "admin_level": "4",
          "boundary": "administrative",
          "def:highway=footway;access:bicycle": "yes",
          "is_in:country_code": "AU",
          "name": "Western Australia",
          "place": "state",
          "ref": "WA",
          "source:name:br": "ofis publik ar brezhoneg",
          "state_code": "WA",
          "timezone": "Australia/Perth",
          "type": "boundary",
          "website": "https://www.wa.gov.au/",
          "website:tourism": "http://westernaustralia.com/",
          "wikidata": "Q3206",
          "wikipedia": "en:Western Australia"
        }
      }
      
        ]
      }      
    "#;

    use super::OverpassResponse;

    #[test]
    fn element_from_response() {
        let result: OverpassResponse = serde_json::from_str(RESPONSE).unwrap();
        assert_eq!(result.elements.len(), 3);
        let element = result.elements.first().unwrap();
        assert!(element.geometry.is_some());
    }
}
