use serde::{Deserialize, Serialize};

/// Access variants from <https://wiki.openstreetmap.org/wiki/Key:access#List_of_possible_values>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
