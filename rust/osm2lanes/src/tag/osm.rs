use crate::tag::{TagKey, Tags};

pub const HIGHWAY: TagKey = TagKey::from("highway");

pub enum Highway {
    Classified(HighwayImportance),
    Link(HighwayImportance),
    Lifecycle(Lifecycle),
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

#[derive(PartialEq, PartialOrd)]
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

pub enum Lifecycle {
    Construction,
    Proposed,
}

impl std::fmt::Display for Lifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Construction => write!(f, "construction"),
            Self::Proposed => write!(f, "proposed"),
        }
    }
}

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

impl std::str::FromStr for Highway {
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
            "construction" => Self::Lifecycle(Lifecycle::Construction),
            "proposed" => Self::Lifecycle(Lifecycle::Proposed),
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

impl std::fmt::Display for Highway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Classified(importance) => write!(f, "{}", importance),
            Self::Link(importance) => write!(f, "{}_link", importance),
            Self::Lifecycle(v) => write!(f, "{}", v),
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

impl Highway {
    /// Get Highway From Tags
    ///
    /// If highway missing return None
    /// If highway unknown return the unknown value
    pub fn from_tags(tags: &Tags) -> Result<Self, Option<String>> {
        tags.get(HIGHWAY)
            .ok_or(None)
            .and_then(|s| s.parse().map_err(Some))
    }

    /// Is Highway Predominantly Motorized
    pub const fn is_road(&self) -> bool {
        matches!(
            self,
            Highway::Classified(_)
                | Highway::Link(_)
                | Highway::Residential
                | Highway::Service
                | Highway::Unclassified
                | Highway::UnknownRoad
        )
    }

    /// Is Highway Predominantly Non-Motorized
    pub const fn is_non_motorized(&self) -> bool {
        matches!(
            self,
            Self::Cycleway
                | Self::Footway
                | Self::Path
                | Self::Pedestrian
                | Self::Steps
                | Self::Track
        )
    }
}
