use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

/// Access variants from <https://wiki.openstreetmap.org/wiki/Key:access#List_of_possible_values>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(IntoStaticStr, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Access {
    Yes,
    No,
    Private,
    Permissive,
    Permit,
    Destination,
    Delivery,
    Customers,
    Designated,
}
