use serde::{Deserialize, Serialize};

use crate::{TagKey, Tags};

pub const ONEWAY: TagKey = TagKey::from_static("oneway");
pub const HIGHWAY: TagKey = TagKey::from_static("highway");
const CONSTRUCTION: TagKey = TagKey::from_static("construction");
const PROPOSED: TagKey = TagKey::from_static("proposed");
pub const LIFECYCLE: [TagKey; 3] = [HIGHWAY, CONSTRUCTION, PROPOSED];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HighwayType {
    Classified(HighwayImportance),
    Link(HighwayImportance),
    NonTravel(NonTravel),
    // Roads
    Residential,
    Service,
    Unclassified,
    UnknownRoad, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Droad
    // Mixed
    Track,
    LivingStreet,
    // Motorized
    BusGuideway,
    // Non-motorized
    Bridleway,
    Corridor,
    Cycleway,
    Footway,
    Path,
    Pedestrian,
    Steps,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum HighwayImportance {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
}

impl std::fmt::Display for HighwayImportance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Motorway => write!(f, "motorway"),
            Self::Trunk => write!(f, "trunk"),
            Self::Primary => write!(f, "primary"),
            Self::Secondary => write!(f, "secondary"),
            Self::Tertiary => write!(f, "tertiary"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum NonTravel {
    Escape,
    Raceway,
}

impl std::fmt::Display for NonTravel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Escape => write!(f, "escape"),
            Self::Raceway => write!(f, "raceway"),
        }
    }
}

impl std::str::FromStr for HighwayType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "motorway" => Self::Classified(HighwayImportance::Motorway),
            "trunk" => Self::Classified(HighwayImportance::Trunk),
            "primary" => Self::Classified(HighwayImportance::Primary),
            "secondary" => Self::Classified(HighwayImportance::Secondary),
            "tertiary" => Self::Classified(HighwayImportance::Tertiary),
            "motorway_link" => Self::Link(HighwayImportance::Motorway),
            "trunk_link" => Self::Link(HighwayImportance::Trunk),
            "primary_link" => Self::Link(HighwayImportance::Primary),
            "secondary_link" => Self::Link(HighwayImportance::Secondary),
            "tertiary_link" => Self::Link(HighwayImportance::Tertiary),
            "raceway" => Self::NonTravel(NonTravel::Raceway),
            "escape" => Self::NonTravel(NonTravel::Escape),
            "bridleway" => Self::Bridleway,
            "bus_guideway" => Self::BusGuideway,
            "corridor" => Self::Corridor,
            "cycleway" => Self::Cycleway,
            "footway" => Self::Footway,
            "living_street" => Self::LivingStreet,
            "path" => Self::Path,
            "pedestrian" => Self::Pedestrian,
            "residential" => Self::Residential,
            "road" => Self::UnknownRoad,
            "service" => Self::Service,
            "steps" => Self::Steps,
            "track" => Self::Track,
            "unclassified" => Self::Unclassified,
            _ => return Err(s.to_owned()),
        })
    }
}

impl std::fmt::Display for HighwayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Classified(importance) => write!(f, "{}", importance),
            Self::Link(importance) => write!(f, "{}_link", importance),
            Self::NonTravel(v) => write!(f, "{}", v),
            Self::Bridleway => write!(f, "bridleway"),
            Self::BusGuideway => write!(f, "bus_guideway"),
            Self::Corridor => write!(f, "corridor"),
            Self::Cycleway => write!(f, "cycleway"),
            Self::Footway => write!(f, "footway"),
            Self::LivingStreet => write!(f, "living_street"),
            Self::Path => write!(f, "path"),
            Self::Pedestrian => write!(f, "pedestrian"),
            Self::Residential => write!(f, "residential"),
            Self::Service => write!(f, "service"),
            Self::Steps => write!(f, "steps"),
            Self::Track => write!(f, "track"),
            Self::Unclassified => write!(f, "unclassified"),
            Self::UnknownRoad => write!(f, "road"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Lifecycle {
    Active,
    Construction,
    Proposed,
}
impl Default for Lifecycle {
    fn default() -> Self {
        Self::Active
    }
}
fn is_default<T>(v: &T) -> bool
where
    T: PartialEq + Default,
{
    T::default().eq(v)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Highway {
    #[serde(
        serialize_with = "serialize_display",
        deserialize_with = "deserialize_from_str"
    )]
    highway: HighwayType,
    #[serde(default, skip_serializing_if = "is_default")]
    lifecycle: Lifecycle,
}
fn serialize_display<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: serde::Serializer,
{
    serializer.collect_str(value)
}
fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
}

impl std::fmt::Display for Highway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.highway)
    }
}

impl Highway {
    /// Get Highway From Tags
    ///
    /// # Errors
    ///
    /// If highway missing return None
    /// If highway unknown return the unknown value
    pub fn from_tags(tags: &Tags) -> Result<Self, Option<String>> {
        tags.get(HIGHWAY).ok_or(None).and_then(|s| match s {
            "construction" => {
                let highway = tags
                    .get(CONSTRUCTION)
                    .map_or(Ok(HighwayType::UnknownRoad), str::parse)
                    .map_err(Some)?;
                Ok(Self {
                    highway,
                    lifecycle: Lifecycle::Construction,
                })
            },
            "proposed" => {
                let highway = tags
                    .get(PROPOSED)
                    .map_or(Ok(HighwayType::UnknownRoad), str::parse)
                    .map_err(Some)?;
                Ok(Self {
                    highway,
                    lifecycle: Lifecycle::Proposed,
                })
            },
            s => {
                let highway = s.parse().map_err(Some)?;
                Ok(Self {
                    highway,
                    lifecycle: Lifecycle::Active,
                })
            },
        })
    }

    /// Active Highway
    #[must_use]
    pub fn active(r#type: HighwayType) -> Self {
        Self {
            highway: r#type,
            lifecycle: Lifecycle::Active,
        }
    }

    /// Is Highway Construction
    #[must_use]
    pub fn is_construction(&self) -> bool {
        matches!(
            self,
            Highway {
                lifecycle: Lifecycle::Construction,
                ..
            }
        )
    }

    /// Is Highway Proposed
    #[must_use]
    pub fn is_proposed(&self) -> bool {
        matches!(
            self,
            Highway {
                lifecycle: Lifecycle::Proposed,
                ..
            }
        )
    }

    /// The type of the highway, independent from its lifecycle
    #[must_use]
    pub fn r#type(&self) -> HighwayType {
        self.highway
    }

    /// Is Highway Supported
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        self.is_supported_road() || self.is_supported_non_motorized()
    }

    /// Is Highway Supported and Predominantly Motorized
    #[must_use]
    pub const fn is_supported_road(&self) -> bool {
        matches!(
            self,
            Highway {
                highway: HighwayType::Classified(_)
                    | HighwayType::Link(_)
                    | HighwayType::Residential
                    | HighwayType::Service
                    | HighwayType::Unclassified
                    | HighwayType::UnknownRoad,
                lifecycle: Lifecycle::Active | Lifecycle::Construction,
            }
        )
    }

    /// Is Highway Supported and Predominantly Non-Motorized
    #[must_use]
    pub const fn is_supported_non_motorized(&self) -> bool {
        matches!(
            self,
            Highway {
                highway: HighwayType::Cycleway
                    | HighwayType::Footway
                    | HighwayType::Path
                    | HighwayType::Pedestrian
                    | HighwayType::Steps
                    | HighwayType::Track,
                lifecycle: Lifecycle::Active,
            }
        )
    }
}
