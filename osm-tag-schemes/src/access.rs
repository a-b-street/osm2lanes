use strum::{EnumString, IntoStaticStr};

/// Access variants from <https://wiki.openstreetmap.org/wiki/Key:access#List_of_possible_values>
#[derive(Clone, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
