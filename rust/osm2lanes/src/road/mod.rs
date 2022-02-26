use serde::{Deserialize, Serialize};

use crate::tag::Highway;
use crate::{Locale, Metre};

mod lane;
pub use lane::{Lane, LaneDesignated, LaneDirection, LanePrintable};

mod marking;
pub use marking::{Marking, MarkingColor, MarkingStyle, Markings};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub lanes: Vec<Lane>,
    pub highway: Highway,
}

impl Road {
    pub fn has_separators(&self) -> bool {
        self.lanes.iter().any(|lane| lane.is_separator())
    }
}

impl Road {
    /// Width in metres
    pub fn width(&self, locale: &Locale) -> Metre {
        self.lanes
            .iter()
            .map(|lane| lane.width(locale))
            .sum::<Metre>()
    }
}
