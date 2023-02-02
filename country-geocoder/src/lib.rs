use geo::{Contains, MultiPolygon, Point};
use geojson::de::deserialize_geometry;
use serde::Deserialize;

/// Calculates which country a point is located in, without any external network lookups.
pub struct CountryGeocoder {
    countries: Vec<Country>,
}

#[derive(Deserialize)]
struct Country {
    #[serde(deserialize_with = "deserialize_geometry")]
    geometry: MultiPolygon<f64>,
    iso_a2: String,
    left_handed: bool,
}

impl CountryGeocoder {
    /// Loads the offline geocoder from built-in data.
    pub fn new() -> Self {
        let raw = include_str!("../data.geojson");
        let countries: Vec<Country> =
            geojson::de::deserialize_feature_collection_str_to_vec(raw).unwrap();
        Self { countries }
    }

    /// Returns `true` if the point is located in a country with left-handed driving. If the point
    /// isn't located in any country, returns `None`.
    pub fn drives_on_left(&self, pt: Point) -> Option<bool> {
        self.lookup(pt).map(|c| c.left_handed)
    }

    /// Returns the [two-letter ISO country code](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2)
    /// where the point is located, or `None` if the point isn't within any country's boundary.
    pub fn iso_a2(&self, pt: Point) -> Option<&str> {
        self.lookup(pt).map(|c| c.iso_a2.as_str())
    }

    fn lookup(&self, pt: Point) -> Option<&Country> {
        self.countries
            .iter()
            .find(|country| country.geometry.contains(&pt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup() {
        let geocoder = CountryGeocoder::new();

        for (lon, lat, iso_a2, drives_on_left) in [
            (174.8376260, -36.9361876, "NZ", true),
            (-2.7003165, 52.0566641, "GB", true),
            (-97.7078040, 30.4173778, "US", false),
            (17.0371953, 51.1128197, "PL", false),
        ] {
            let pt = Point::new(lon, lat);
            assert_eq!(geocoder.iso_a2(pt), Some(iso_a2));
            assert_eq!(geocoder.drives_on_left(pt), Some(drives_on_left));
        }

        // The deep blue
        assert_eq!(geocoder.iso_a2(Point::new(0.960324, 56.577476)), None);
        assert_eq!(
            geocoder.drives_on_left(Point::new(0.960324, 56.577476)),
            None
        );
    }
}
