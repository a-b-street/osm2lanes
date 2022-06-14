#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct Metre(f64);

impl Metre {
    pub const MIN: Metre = Self(f64::MIN);
    pub const MAX: Metre = Self(f64::MAX);

    #[must_use]
    pub const fn new(val: f64) -> Self {
        Self(val)
    }

    #[must_use]
    pub const fn val(&self) -> f64 {
        self.0
    }
}

impl std::ops::Add for Metre {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}
impl std::ops::AddAssign for Metre {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0 + other.0);
    }
}
impl std::ops::Mul<Metre> for f64 {
    type Output = Metre;
    fn mul(self, other: Metre) -> Self::Output {
        Metre::new(self * other.val())
    }
}
impl std::iter::Sum for Metre {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Metre>,
    {
        Self(iter.map(|m| m.0).sum())
    }
}

/// Vehicle Speed, used for speed limits and similar.
// TODO: This part of the API may need to be revisited entirely
// It is unclear whether a speed unit is needed per lane,
// or if it could be provided for the entire road.
// Data consumers may also want the km/h value
// regardless of the unit used in the highway's locale,
// in order to do distance over time calculations.
// The serialization of this is clunky and leaves a lot to be desired.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Speed {
    Kph(f64),
    Mph(f64),
    Knots(f64),
}

impl Speed {
    #[must_use]
    pub fn kph(&self) -> f64 {
        match self {
            Self::Kph(val) => *val,
            Self::Mph(val) => 1.60934_f64 * val,
            Self::Knots(val) => 1.852_f64 * val,
        }
    }
}

#[derive(Debug)]
pub enum SpeedError {
    Empty,
    Parse(std::num::ParseFloatError),
    UnknownUnit(String),
    OutOfRange,
}

impl std::fmt::Display for SpeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty"),
            Self::Parse(e) => e.fmt(f),
            Self::UnknownUnit(unit) => write!(f, "unknown unit '{unit}'"),
            Self::OutOfRange => write!(f, "out of range"),
        }
    }
}

impl std::error::Error for SpeedError {}

impl From<std::num::ParseFloatError> for SpeedError {
    fn from(e: std::num::ParseFloatError) -> Self {
        SpeedError::Parse(e)
    }
}

impl std::str::FromStr for Speed {
    type Err = SpeedError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(SpeedError::Empty);
        }
        let speed = match s.split_once(' ') {
            None => Self::Kph(s.parse()?),
            Some((s, "mph")) => Self::Mph(s.parse()?),
            Some((s, "knots")) => Self::Knots(s.parse()?),
            Some((_, unit)) => return Err(SpeedError::UnknownUnit(unit.to_owned())),
        };
        if speed.kph() < 0_f64 || speed.kph() > 300_f64 {
            return Err(SpeedError::OutOfRange);
        }
        Ok(speed)
    }
}

impl std::fmt::Display for Speed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Kph(v) => write!(f, "{}", v),
            Self::Mph(v) => write!(f, "{} mph", v),
            Self::Knots(v) => write!(f, "{} knots", v),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Speed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        speed::serialize(self, serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Speed {
    fn deserialize<D>(deserializer: D) -> Result<Speed, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        speed::deserialize(deserializer)
    }
}

mod speed {
    use std::num::ParseFloatError;

    #[derive(Debug, PartialEq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    struct SpeedStruct {
        unit: SpeedUnit,
        value: f64,
    }

    #[derive(Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    enum SpeedUnit {
        Kph,
        #[cfg(feature = "serde")]
        Mph,
        #[cfg(feature = "serde")]
        Knots,
    }

    impl std::str::FromStr for SpeedStruct {
        type Err = ParseFloatError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self {
                unit: SpeedUnit::Kph,
                value: s.parse()?,
            })
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize<S>(speed: &super::Speed, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::Serialize;

        use super::Speed;
        match speed {
            Speed::Kph(v) => serializer.serialize_f64(*v),
            Speed::Mph(v) => SpeedStruct {
                unit: SpeedUnit::Mph,
                value: *v,
            }
            .serialize(serializer),
            Speed::Knots(v) => SpeedStruct {
                unit: SpeedUnit::Knots,
                value: *v,
            }
            .serialize(serializer),
        }
    }

    // https://serde.rs/string-or-struct.html

    #[cfg(feature = "serde")]
    struct FloatOrStruct;

    #[cfg(feature = "serde")]
    impl<'de> serde::de::Visitor<'de> for FloatOrStruct {
        type Value = SpeedStruct;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("number or map")
        }

        fn visit_f64<E>(self, value: f64) -> Result<SpeedStruct, E>
        where
            E: serde::de::Error,
        {
            Ok(SpeedStruct {
                unit: SpeedUnit::Kph,
                value,
            })
        }

        fn visit_u32<E>(self, value: u32) -> Result<SpeedStruct, E>
        where
            E: serde::de::Error,
        {
            Ok(SpeedStruct {
                unit: SpeedUnit::Kph,
                value: f64::from(value),
            })
        }

        // TODO: why is this needed if u32 is enough?
        fn visit_u64<E>(self, value: u64) -> Result<SpeedStruct, E>
        where
            E: serde::de::Error,
        {
            let value = u32::try_from(value).unwrap();
            let value = f64::from(value);
            Ok(SpeedStruct {
                unit: SpeedUnit::Kph,
                value,
            })
        }

        fn visit_i32<E>(self, value: i32) -> Result<SpeedStruct, E>
        where
            E: serde::de::Error,
        {
            Ok(SpeedStruct {
                unit: SpeedUnit::Kph,
                value: f64::from(value),
            })
        }

        // TODO: why is this needed if i32 is enough?
        fn visit_i64<E>(self, value: i64) -> Result<SpeedStruct, E>
        where
            E: serde::de::Error,
        {
            let value = i32::try_from(value).unwrap();
            let value = f64::from(value);
            Ok(SpeedStruct {
                unit: SpeedUnit::Kph,
                value,
            })
        }

        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            serde::Deserialize::deserialize(serde::de::value::MapAccessDeserializer::new(map))
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<super::Speed, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: SpeedStruct = deserializer.deserialize_any(FloatOrStruct)?;
        Ok(match s {
            SpeedStruct {
                unit: SpeedUnit::Kph,
                value,
            } => super::Speed::Kph(value),
            SpeedStruct {
                unit: SpeedUnit::Mph,
                value,
            } => super::Speed::Mph(value),
            SpeedStruct {
                unit: SpeedUnit::Knots,
                value,
            } => super::Speed::Knots(value),
        })
    }

    #[cfg(test)]
    mod tests {
        use crate::metric::speed::{SpeedStruct, SpeedUnit};

        #[test]
        fn test_speed() {
            let speed_kph_struct = (
                SpeedStruct {
                    unit: SpeedUnit::Kph,
                    value: 1.0,
                },
                r#"{"unit":"kph","value":1.0}"#,
            );

            assert_eq!(
                serde_json::to_string(&speed_kph_struct.0).unwrap(),
                speed_kph_struct.1,
            );
            assert_eq!(
                speed_kph_struct.0,
                serde_json::from_str(speed_kph_struct.1).unwrap()
            );
        }
    }
}

#[allow(clippy::similar_names)]
#[cfg(test)]
mod tests {
    use crate::metric::Speed;

    #[test]
    fn test_speed() {
        let speed_kph_str = (Speed::Kph(1.0), "1.0");
        let speed_kph_struct = (Speed::Kph(2.0), r#"{ "unit": "kph", "value": 2.0 }"#);
        let speed_mph_struct = (Speed::Mph(3.0), r#"{ "unit": "mph", "value": 3.0 }"#);
        let speed_knots_struct = (Speed::Knots(4.0), r#"{ "unit": "knots", "value": 4.0 }"#);

        assert_eq!(
            serde_json::to_string(&speed_kph_str.0).unwrap(),
            speed_kph_str.1
        );
        assert_eq!(
            serde_json::to_string(&speed_mph_struct.0).unwrap(),
            r#"{"unit":"mph","value":3.0}"#
        );

        assert_eq!(
            speed_kph_str.0,
            serde_json::from_str(speed_kph_str.1).unwrap()
        );
        assert_eq!(
            speed_kph_struct.0,
            serde_json::from_str(speed_kph_struct.1).unwrap()
        );
        assert_eq!(
            speed_mph_struct.0,
            serde_json::from_str(speed_mph_struct.1).unwrap()
        );
        assert_eq!(
            speed_knots_struct.0,
            serde_json::from_str(speed_knots_struct.1).unwrap()
        );

        assert_eq!(
            speed_kph_str.0,
            serde_json::from_str(&serde_json::to_string(&speed_kph_str.0).unwrap()).unwrap(),
        );
        assert_eq!(
            speed_kph_struct.0,
            serde_json::from_str(&serde_json::to_string(&speed_kph_struct.0).unwrap()).unwrap(),
        );
        assert_eq!(
            speed_mph_struct.0,
            serde_json::from_str(&serde_json::to_string(&speed_mph_struct.0).unwrap()).unwrap(),
        );
        assert_eq!(
            speed_knots_struct.0,
            serde_json::from_str(&serde_json::to_string(&speed_knots_struct.0).unwrap()).unwrap(),
        );
    }
}
