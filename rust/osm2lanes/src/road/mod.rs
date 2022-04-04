use serde::{Deserialize, Serialize};

use crate::locale::Locale;
use crate::metric::Metre;
use crate::tag::Highway;

mod lane;
pub use lane::{Designated, Direction, Lane, Printable};

mod marking;
pub use marking::{Color, Marking, Markings, Style};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub lanes: Vec<Lane>,
    pub highway: Highway,
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
            .map(|lane| lane.width(locale))
            .sum::<Metre>()
    }
}
