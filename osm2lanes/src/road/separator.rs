use serde::{Deserialize, Serialize};

use super::Printable;
use crate::locale::Locale;
use crate::metric::Metre;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Semantic {
    Buffer,
    Centre,
    Hard,
    Kerb,
    Lane,
    Modal,
    Shoulder,
    Verge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Markings(Vec<Marking>);

impl Markings {
    #[must_use]
    pub fn new(markings: Vec<Marking>) -> Self {
        Self(markings)
    }

    /// Flip left and right, reverses the order of markings and inverts them in place.
    pub fn flip(&mut self) {
        self.0.reverse();
        for marking in &mut self.0 {
            marking.invert();
        }
    }

    /// Width in metres
    #[must_use]
    pub fn width(&self, _locale: &Locale) -> Metre {
        self.0
            .iter()
            .map(|marking| marking.width.unwrap_or(Marking::DEFAULT_WIDTH))
            .sum::<Metre>()
    }
}

impl std::ops::Deref for Markings {
    type Target = Vec<Marking>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marking {
    pub style: Style,
    pub width: Option<Metre>,
    pub color: Option<Color>,
}

impl Marking {
    pub const DEFAULT_WIDTH: Metre = Metre::new(0.2);
    pub const DEFAULT_SPACE: Metre = Metre::new(0.1);

    pub fn invert(&mut self) {
        self.style = self.style.opposite();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    #[serde(rename = "solid_line")]
    SolidLine,
    #[serde(rename = "broken_line")]
    BrokenLine,
    #[serde(rename = "dashed_line")]
    DashedLine,
    #[serde(rename = "dotted_line")]
    DottedLine,
    // #[serde(rename = "gore_chevron")]
    // GoreChevron,
    // #[serde(rename = "diagnoal_hatched")]
    // DiagonalCross,
    // #[serde(rename = "criss_cross")]
    // CrissCross,
    // #[serde(rename = "solid_fill")]
    // SolidFill,
    #[serde(rename = "no_fill")]
    NoFill,
    // up and down are left to right
    #[serde(rename = "kerb_up")]
    KerbUp,
    #[serde(rename = "kerb_down")]
    KerbDown,
}

impl Style {
    /// UTF8 representation of markings
    #[must_use]
    pub const fn as_utf8(&self) -> char {
        match self {
            Self::SolidLine => '|',
            Self::BrokenLine => '¦',
            Self::DashedLine => ':',
            Self::DottedLine => '᛫',
            Self::KerbDown => '\\',
            Self::KerbUp => '/',
            Self::NoFill => ' ',
        }
    }
    /// Opposite marking style
    #[must_use]
    pub const fn opposite(&self) -> Self {
        match self {
            Self::SolidLine => Self::SolidLine,
            Self::BrokenLine => Self::BrokenLine,
            Self::DashedLine => Self::DashedLine,
            Self::DottedLine => Self::DottedLine,
            Self::KerbDown => Self::KerbUp,
            Self::KerbUp => Self::KerbDown,
            Self::NoFill => Self::NoFill,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    #[serde(rename = "white")]
    White,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "green")]
    Green,
}

impl Printable for Color {
    fn as_ascii(&self) -> char {
        match self {
            Self::White => 'w',
            Self::Yellow => 'y',
            Self::Red => 'r',
            Self::Green => 'g',
        }
    }
    fn as_utf8(&self) -> char {
        self.as_ascii()
    }
}
