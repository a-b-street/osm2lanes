use serde::{Deserialize, Serialize};

use crate::locale::Locale;
use crate::metric::Metre;
use crate::tag_keys::Highway;

mod lane;
pub use lane::{AccessAndDirection, AccessByType, Designated, Direction, Lane, Printable};

mod separator;
pub use separator::{Color, Marking, Markings, Semantic, Style};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,

    #[serde(flatten)]
    pub highway: Highway,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracktype: Option<TrackType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smoothness: Option<Smoothness>,

    pub lanes: Vec<Lane>,
}

impl Road {
    #[must_use]
    pub fn has_separators(&self) -> bool {
        self.lanes.iter().any(Lane::is_separator)
    }
}

impl Road {
    /// Width in metres
    #[must_use]
    pub fn width(&self, locale: &Locale) -> Metre {
        self.lanes
            .iter()
            .map(|lane| lane.width(locale, self.highway.r#type()))
            .sum::<Metre>()
    }
}
