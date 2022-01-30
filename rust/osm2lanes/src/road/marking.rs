use serde::{Deserialize, Serialize};

use crate::Metre;

use super::LanePrintable;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marking {
    pub style: MarkingStyle,
    pub width: Option<Metre>,
    pub color: Option<MarkingColor>,
}

impl Marking {
    pub const DEFAULT_WIDTH: Metre = Metre::new(0.2);
    pub const DEFAULT_SPACE: Metre = Metre::new(0.1);
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarkingStyle {
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarkingColor {
    #[serde(rename = "white")]
    White,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "red")]
    Red,
}

impl MarkingStyle {
    pub fn as_utf8(&self) -> char {
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
}

impl LanePrintable for MarkingColor {
    fn as_ascii(&self) -> char {
        match self {
            Self::White => 'w',
            Self::Yellow => 'y',
            Self::Red => 'r',
        }
    }
    fn as_utf8(&self) -> char {
        self.as_ascii()
    }
}
