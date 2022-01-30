use serde::{Deserialize, Serialize};

use crate::{Locale, Metre};

mod lane;
pub use lane::{Lane, LaneDesignated, LaneDirection, LanePrintable};

mod marking;
pub use marking::{Marking, MarkingColor, MarkingStyle};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub lanes: Vec<Lane>,
}

impl Road {
    pub fn has_separators(&self) -> bool {
        self.lanes.iter().any(|lane| lane.is_separator())
    }
}

impl Road {
    pub fn total_width(&self, locale: &Locale) -> Metre {
        self.lanes
            .iter()
            .map(|lane| match lane {
                Lane::Separator { markings } => markings
                    .iter()
                    .map(|marking| marking.width.unwrap_or(Marking::DEFAULT_WIDTH))
                    .sum::<Metre>(),
                Lane::Travel { designated, .. } => locale.default_width(designated),
                _ => Lane::DEFAULT_WIDTH,
            })
            .sum::<Metre>()
    }
}
