use geo::{Contains, Point};
use geojson::{quick_collection, GeoJson};

/// Returns true if the point is located in a country with left-handed driving.
pub fn is_left_handed(pt: Point) -> bool {
    let raw = include_str!("../data.geojson");
    let geojson = raw.parse::<GeoJson>().unwrap();
    let features = quick_collection(&geojson).unwrap();

    for feature in features {
        if feature.contains(&pt) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country_lookup() {
        // New Zealand
        assert!(is_left_handed(Point::new(174.8376260, -36.9361876)));
        // UK
        assert!(is_left_handed(Point::new(-2.7003165, 52.0566641)));

        // US
        assert!(!is_left_handed(Point::new(-97.7078040, 30.4173778)));
        // Poland
        assert!(!is_left_handed(Point::new(17.0371953, 51.1128197)));
    }
}
