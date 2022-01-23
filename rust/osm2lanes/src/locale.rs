use serde::{Deserialize, Serialize};

/// Context about the place where an OSM way exists.
#[derive(Debug, Serialize, Deserialize)]
pub struct Locale {
    pub driving_side: DrivingSide,
    /// When sidewalks are not explicitly tagged on a way, should sidewalks or shoulder lanes be
    /// placed anyway based on heuristics?
    pub infer_sidewalks: bool,
}

impl Locale {
    pub fn builder() -> Config {
        Config::default()
    }
}

/// Configuration to build locale, context about the place where an OSM way exists.
#[derive(Default)]
pub struct Config {
    way_id: Option<u32>,
    _iso_3166_1_alpha_2: Option<String>,
    _iso_3166_1_alpha_3: Option<String>,
    _iso_3166_2: Option<String>,
    driving_side: Option<DrivingSide>,
    infer_sidewalks: Option<bool>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn way_id(mut self, id: u32) -> Self {
        self.way_id = Some(id);
        todo!();
    }

    pub fn iso_3166(self, _code: &str) -> Self {
        todo!();
    }

    pub fn driving_side(mut self, side: DrivingSide) -> Self {
        self.driving_side = Some(side);
        self
    }

    pub fn infer_sidewalks(mut self, infer: bool) -> Self {
        self.infer_sidewalks = Some(infer);
        self
    }

    pub fn build(&self) -> Locale {
        // TODO, more business logic
        Locale {
            driving_side: self.driving_side.unwrap_or(DrivingSide::Right),
            infer_sidewalks: self.infer_sidewalks.unwrap_or(true), // TODO?
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
