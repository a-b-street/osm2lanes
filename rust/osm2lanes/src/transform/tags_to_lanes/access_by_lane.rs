#[derive(Debug)]
pub(in crate::transform::tags_to_lanes) enum Access {
    None,
    No,
    Yes,
    Designated,
}

impl std::str::FromStr for Access {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Self::None),
            "no" => Ok(Self::No),
            "yes" => Ok(Self::Yes),
            "designated" => Ok(Self::Designated),
            _ => Err(s.to_owned()),
        }
    }
}

impl Access {
    pub(in crate::transform::tags_to_lanes) fn split(lanes: &str) -> Result<Vec<Self>, String> {
        lanes.split('|').map(str::parse).collect()
    }
}
