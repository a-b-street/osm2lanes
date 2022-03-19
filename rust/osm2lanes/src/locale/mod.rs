use std::collections::HashMap;

pub use celes::Country;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::road::{LaneDesignated, MarkingColor};
use crate::Metre;

#[derive(Serialize, Deserialize, Debug)]
struct CenterLineStyle(HashMap<String, MarkingColor>);
const CENTER_LINE_YML: &str = include_str!("country_metadata/center_line_style.yml");
static CENTER_LINE_DATA: Lazy<CenterLineStyle> =
    Lazy::new(|| serde_yaml::from_str(CENTER_LINE_YML).unwrap());

/// Context about the place where an OSM way exists.
#[derive(Debug, Serialize, Deserialize)]
pub struct Locale {
    /// The ISO 3166 Country
    pub country: Option<Country>,
    pub iso_3166_2_subdivision: Option<String>,
    /// The driving side
    pub driving_side: DrivingSide,
}

impl Locale {
    pub fn builder() -> Config {
        Config::default()
    }
    pub fn travel_width(&self, designated: &LaneDesignated) -> Metre {
        match designated {
            LaneDesignated::Motor | LaneDesignated::Bus => Metre::new(3.5),
            LaneDesignated::Foot => Metre::new(2.5),
            LaneDesignated::Bicycle => Metre::new(2.0),
        }
    }
    pub fn lane_separator_color(&self) -> Option<MarkingColor> {
        if let Some(country) = &self.country {
            CENTER_LINE_DATA.0.get(country.alpha2).copied()
        } else {
            None
        }
    }
}

/// Configuration to build locale, context about the place where an OSM way exists.
#[derive(Default)]
pub struct Config {
    way_id: Option<u64>,
    iso_3166_1_alpha_2: Option<String>,
    iso_3166_1_alpha_3: Option<String>,
    iso_3166_2_subdivision: Option<String>,
    driving_side: Option<DrivingSide>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn way_id(mut self, id: u64) -> Self {
        self.way_id = Some(id);
        todo!();
    }

    /// Assign ISO-3166
    ///
    /// Accepts any of ISO-3166-1 alpha-2,  ISO-3166-1 alpha-3, or ISO-3166-2 codes
    pub fn iso_3166(mut self, code: &str) -> Self {
        if code.len() == 2 {
            self.iso_3166_1_alpha_2 = Some(code.to_owned());
        } else if code.len() == 3 {
            self.iso_3166_1_alpha_3 = Some(code.to_owned());
        } else if let Some((alpha_2, subdivision)) = code.split_once('-') {
            self.iso_3166_1_alpha_2 = Some(alpha_2.to_owned());
            self.iso_3166_2_subdivision = Some(subdivision.to_owned());
        } else {
            todo!();
        }
        self
    }

    pub fn iso_3166_option(mut self, code: Option<&str>) -> Self {
        if let Some(code) = code {
            self = self.iso_3166(code)
        }
        self
    }

    pub fn driving_side(mut self, side: DrivingSide) -> Self {
        self.driving_side = Some(side);
        self
    }

    pub fn build(&self) -> Locale {
        // TODO, more business logic
        let country = match (
            &self.iso_3166_1_alpha_2,
            &self.iso_3166_1_alpha_3,
            &self.iso_3166_2_subdivision,
        ) {
            (None, None, _) => None,
            (Some(c), None, _) => Country::from_alpha2(&c).ok(),
            (None, Some(c), _) => Country::from_alpha3(&c).ok(),
            (Some(_), Some(_), _) => unimplemented!(),
        };
        Locale {
            country,
            iso_3166_2_subdivision: self.iso_3166_2_subdivision.clone(),
            driving_side: self.driving_side.unwrap_or(DrivingSide::Right),
        }
    }
}

/// Do vehicles travel on the right or left side of a road?
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DrivingSide {
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "left")]
    Left,
}

impl DrivingSide {
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

#[cfg(test)]
mod tests {
    use celes::Country;

    use crate::{DrivingSide, Locale};

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
