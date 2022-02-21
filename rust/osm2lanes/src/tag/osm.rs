use crate::tag::{TagKey, Tags};

pub const HIGHWAY: TagKey = TagKey::from("highway");

pub enum Highway {
    Road(Road),
    Link(Road),
    Bridleway,
    BusGuideway,
    Construction,
    Corridor,
    Cycleway,
    Escape,
    Footway,
    LivingStreet,
    Path,
    Pedestrian,
    Proposed,
    Raceway,
    Residential,
    Service,
    Steps,
    Track,
}

pub enum Road {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
    Unclassified,
    Unknown, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Droad
}

impl std::fmt::Display for Road {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Road::Motorway => write!(f, "motorway"),
            Road::Trunk => write!(f, "trunk"),
            Road::Primary => write!(f, "primary"),
            Road::Secondary => write!(f, "secondary"),
            Road::Tertiary => write!(f, "tertiary"),
            Road::Unclassified => write!(f, "unclassified"),
            Road::Unknown => write!(f, "road"),
        }
    }
}

impl std::str::FromStr for Highway {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "motorway" => Self::Road(Road::Motorway),
            "trunk" => Self::Road(Road::Trunk),
            "primary" => Self::Road(Road::Primary),
            "secondary" => Self::Road(Road::Secondary),
            "tertiary" => Self::Road(Road::Tertiary),
            "unclassified" => Self::Road(Road::Unclassified),
            "road" => Self::Road(Road::Unknown),
            "motorway_link" => Self::Link(Road::Motorway),
            "trunk_link" => Self::Link(Road::Trunk),
            "primary_link" => Self::Link(Road::Primary),
            "secondary_link" => Self::Link(Road::Secondary),
            "tertiary_link" => Self::Link(Road::Tertiary),
            "bridleway" => Self::Bridleway,
            "bus_guideway" => Self::BusGuideway,
            "construction" => Self::Construction,
            "corridor" => Self::Corridor,
            "cycleway" => Self::Cycleway,
            "escape" => Self::Escape,
            "footway" => Self::Footway,
            "living_street" => Self::LivingStreet,
            "path" => Self::Path,
            "pedestrian" => Self::Pedestrian,
            "proposed" => Self::Proposed,
            "raceway" => Self::Raceway,
            "residential" => Self::Residential,
            "service" => Self::Service,
            "steps" => Self::Steps,
            "track" => Self::Track,
            _ => return Err(s.to_owned()),
        })
    }
}

impl std::fmt::Display for Highway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Road(road) => write!(f, "{}", road),
            Self::Link(road) => write!(f, "{}_link", road),
            Self::Bridleway => write!(f, "bridleway"),
            Self::BusGuideway => write!(f, "bus_guideway"),
            Self::Construction => write!(f, "construction"),
            Self::Corridor => write!(f, "corridor"),
            Self::Cycleway => write!(f, "cycleway"),
            Self::Escape => write!(f, "escape"),
            Self::Footway => write!(f, "footway"),
            Self::LivingStreet => write!(f, "living_street"),
            Self::Path => write!(f, "path"),
            Self::Pedestrian => write!(f, "pedestrian"),
            Self::Proposed => write!(f, "proposed"),
            Self::Raceway => write!(f, "raceway"),
            Self::Residential => write!(f, "residential"),
            Self::Service => write!(f, "service"),
            Self::Steps => write!(f, "steps"),
            Self::Track => write!(f, "track"),
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
