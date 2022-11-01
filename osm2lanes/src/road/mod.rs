use osm_tag_schemes::{Highway, HighwayType, Lit, Smoothness, TrackType};

use crate::locale::Locale;
use crate::metric::Metre;

mod lane;
pub use lane::{AccessAndDirection, AccessByType, Designated, Direction, Lane, Printable};

mod separator;
pub use separator::{Color, Marking, Markings, Semantic, Style};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Road {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub r#ref: Option<String>,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub highway: Highway,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub lit: Option<Lit>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tracktype: Option<TrackType>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub smoothness: Option<Smoothness>,

    pub lanes: Vec<Lane>,
}

impl Road {
    /// A road without any metadata or lanes filled out
    #[must_use]
    pub fn empty() -> Self {
        Self {
            name: None,
            r#ref: None,
            highway: Highway::active(HighwayType::UnknownRoad),
            lit: None,
            tracktype: None,
            smoothness: None,
            lanes: Vec::new(),
        }
    }

    #[must_use]
    pub fn has_separators(&self) -> bool {
        self.lanes.iter().any(Lane::is_separator)
    }

    /// Width in metres
    #[must_use]
    pub fn width(&self, locale: &Locale) -> Metre {
        self.lanes
            .iter()
            .map(|lane| lane.width(locale, self.highway.r#type()))
            .sum::<Metre>()
    }
}
