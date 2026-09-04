use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

const DISCORD_EPOCH_UNIX_MILLIS: u64 = 1_420_070_400_000;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Snowflake(u64);

impl Snowflake {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn timestamp_unix_millis(self) -> u64 {
        (self.0 >> 22) + DISCORD_EPOCH_UNIX_MILLIS
    }
}

impl Display for Snowflake {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl FromStr for Snowflake {
    type Err = ParseIntError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}

impl Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Snowflake;
    use std::str::FromStr;

    #[test]
    fn timestamp_matches_known_discord_id() {
        let snowflake = Snowflake::from_raw(175928847299117063);
        assert_eq!(snowflake.timestamp_unix_millis(), 1_462_015_105_796);
    }

    #[test]
    fn parses_decimal_text() {
        let snowflake = Snowflake::from_str("175928847299117063");
        assert_eq!(snowflake.map(Snowflake::raw), Ok(175928847299117063));
    }

    #[test]
    fn roundtrips_json_as_a_decimal_string() {
        let snowflake = Snowflake::from_raw(175928847299117063);
        let encoded = serde_json::to_string(&snowflake).unwrap_or_default();
        assert_eq!(encoded, "\"175928847299117063\"");
        let decoded = serde_json::from_str::<Snowflake>(&encoded).ok();
        assert_eq!(decoded, Some(snowflake));
    }
}
