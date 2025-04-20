use std::time::Duration;

use serde::{Deserialize, Deserializer};

pub fn parse_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let unit = &s[s.len() - 1..];
    let value: u64 = s[..s.len() - 1]
        .parse()
        .map_err(|_| "invalid unsigned integer")
        .map_err(serde::de::Error::custom)?;

    match unit {
        "s" => Ok(Duration::from_secs(value)),
        "m" => Ok(Duration::from_secs(value * 60)),
        "h" => Ok(Duration::from_secs(value * 60 * 60)),
        _ => Err(serde::de::Error::custom(format!(
            "invalid duration unit '{}'",
            unit
        ))),
    }
}
