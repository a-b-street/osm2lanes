pub use celes::Country;
use osm_tag_schemes::{HighwayImportance, HighwayType};

use crate::metric::Metre;
use crate::road::{Color, Designated};

/// Context about the place where an OSM way exists.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Locale {
    /// The ISO 3166 Country
    pub country: Option<Country>,
    pub iso_3166_2_subdivision: Option<String>,
    /// The driving side
    pub driving_side: DrivingSide,
}

impl Locale {
    #[must_use]
    pub fn builder() -> Config {
        Config::default()
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn travel_width(&self, designated: &Designated, _highway: HighwayType) -> Metre {
        match designated {
            Designated::Motor | Designated::Bus => {
                let uk = Country::the_united_kingdom_of_great_britain_and_northern_ireland();
                match &self.country {
                    // Guessed, TODO: find DfT source.
                    Some(c) if c == &uk => Metre::new(3.0),
                    // https://puc.overheid.nl/rijkswaterstaat/doc/PUC_125514_31/ section 4.2.5
                    Some(c) if c == &Country::the_netherlands() => Metre::new(3.35),
                    _ => Metre::new(3.5),
                }
            },
            Designated::Foot => Metre::new(2.5),
            Designated::Bicycle => Metre::new(2.0),
        }
    }

    /// Road paint colour separating opposite directions of motor traffic
    /// default is white
    #[must_use]
    pub fn separator_motor_color(&self) -> Color {
        match self
            .country
            .as_ref()
            .map(|c| c.alpha3)
            .and_then(locale_codes::country::lookup)
            .and_then(|c| c.region_code)
            .and_then(locale_codes::region::lookup)
            .map(|region| region.name.as_str())
        {
            Some("Americas") => Color::Yellow,
            Some(_) | None => Color::White,
        }
    }

    /// Road marking width separating opposite directions of motor traffic
    /// default is 0.2, TODO: is this a good default?
    #[must_use]
    pub fn separator_motor_width(&self) -> Metre {
        match &self.country {
            Some(c)
                if c == &Country::the_united_kingdom_of_great_britain_and_northern_ireland() =>
            {
                // https://en.wikisource.org/wiki/Traffic_Signs_Manual/Chapter_5/2009/4
                Metre::new(0.1)
            },
            _ => Metre::new(0.2),
        }
    }

    /// Highway type with no `lanes=*` has:
    /// - (false) one lane with travel in both directions or
    /// - (true) two lanes with travel in opposite directions
    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn has_split_lanes(&self, highway: HighwayType) -> bool {
        matches!(
            highway,
            HighwayType::Classified(
                HighwayImportance::Motorway
                    | HighwayImportance::Trunk
                    | HighwayImportance::Primary
                    | HighwayImportance::Secondary
                    | HighwayImportance::Tertiary,
            ) | HighwayType::Link(
                HighwayImportance::Motorway
                    | HighwayImportance::Trunk
                    | HighwayImportance::Primary
                    | HighwayImportance::Secondary
                    | HighwayImportance::Tertiary,
            ) | HighwayType::Residential
        )
    }

    /// Highway type has shoulder(s) by default
    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn has_shoulder(&self, highway: HighwayType) -> bool {
        matches!(
            highway,
            HighwayType::Classified(
                HighwayImportance::Motorway
                    | HighwayImportance::Trunk
                    | HighwayImportance::Primary
                    | HighwayImportance::Secondary,
            ) | HighwayType::Link(
                HighwayImportance::Motorway
                    | HighwayImportance::Trunk
                    | HighwayImportance::Primary
                    | HighwayImportance::Secondary,
            )
        )
    }
}

/// Configuration to build locale, context about the place where an OSM way exists.
#[derive(Default)]
pub struct Config {
    way_id: Option<u64>,
    iso_3166_1_alpha_2: Option<String>,
    iso_3166_1_alpha_3: Option<String>,
    iso_3166_2_subdivision: Option<String>,
    country: Option<Country>,
    driving_side: Option<DrivingSide>,
}

impl Config {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::missing_panics_doc, clippy::todo)]
    #[must_use]
    pub fn way_id(mut self, id: u64) -> Self {
        self.way_id = Some(id);
        todo!();
    }

    /// Assign ISO-3166
    ///
    /// Accepts any of ISO-3166-1 alpha-2,  ISO-3166-1 alpha-3, or ISO-3166-2 codes
    ///
    /// # Panics
    ///
    /// Cannot determine ISO3166 from code
    /// TODO: this should probably not be a panic
    #[allow(clippy::panic)]
    #[must_use]
    pub fn iso_3166(mut self, code: &str) -> Self {
        if code.len() == 2 {
            self.iso_3166_1_alpha_2 = Some(code.to_owned());
        } else if code.len() == 3 {
            self.iso_3166_1_alpha_3 = Some(code.to_owned());
        } else if let Some((alpha_2, subdivision)) = code.split_once('-') {
            self.iso_3166_1_alpha_2 = Some(alpha_2.to_owned());
            self.iso_3166_2_subdivision = Some(subdivision.to_owned());
        } else {
            panic!("cannot determine ISO 3166 from {code}");
        }
        self
    }

    #[must_use]
    pub fn iso_3166_option(mut self, code: Option<&str>) -> Self {
        if let Some(code) = code {
            self = self.iso_3166(code);
        }
        self
    }

    #[must_use]
    pub fn country(mut self, country: Country) -> Self {
        self.country = Some(country);
        self
    }

    #[must_use]
    pub fn driving_side(mut self, side: DrivingSide) -> Self {
        self.driving_side = Some(side);
        self
    }

    #[must_use]
    pub fn build(&self) -> Locale {
        // TODO, more business logic
        let country = match (
            &self.iso_3166_1_alpha_2,
            &self.iso_3166_1_alpha_3,
            &self.iso_3166_2_subdivision,
            &self.country,
        ) {
            (None, None, _, None) => None,
            (Some(c), None, _, None) => Country::from_alpha2(&c).ok(),
            (None, Some(c), _, None) => Country::from_alpha3(&c).ok(),
            (None, None, _, Some(c)) => Some(*c),
            (None | Some(_), None | Some(_), _, Some(_c)) => unimplemented!(),
            (Some(_), Some(_), _, None) => unimplemented!(),
        };
        Locale {
            country,
            iso_3166_2_subdivision: self.iso_3166_2_subdivision.clone(),
            driving_side: self.driving_side.unwrap_or(DrivingSide::Right),
        }
    }
}

/// Do vehicles travel on the right or left side of a road?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DrivingSide {
    Right,
    Left,
}

impl DrivingSide {
    #[must_use]
    pub fn opposite(&self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }
}

impl std::str::FromStr for DrivingSide {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "right" => Ok(Self::Right),
            "left" => Ok(Self::Left),
            _ => Err(s.to_owned()),
        }
    }
}

impl ToString for DrivingSide {
    fn to_string(&self) -> String {
        match self {
            Self::Right => String::from("right"),
            Self::Left => String::from("left"),
        }
    }
}

#[cfg(test)]
mod tests {
    use celes::Country;

    use crate::locale::{DrivingSide, Locale};

    #[test]
    fn test_locale() {
        let locale = Locale::builder()
            .driving_side(DrivingSide::Right)
            .iso_3166("DE-NW")
            .build();
        assert_eq!(locale.driving_side, DrivingSide::Right);
        assert_eq!(locale.country.unwrap(), Country::germany());
    }
}
