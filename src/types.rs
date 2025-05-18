use std::ops::Deref;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};

/// The name of a currency
#[derive(Debug, Serialize, Deserialize)]
pub struct Currency(String);

#[derive(Debug)]
pub struct CommaDec(Decimal);

impl<'de> serde::Deserialize<'de> for CommaDec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(CommaDec(
            Decimal::from_str_exact(&s).map_err(serde::de::Error::custom)?,
        ))
    }
}

impl Deref for CommaDec {
    type Target = Decimal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<CommaDec> for Decimal {
    fn from(value: CommaDec) -> Self {
        value.0
    }
}

pub fn dec_from_swe_num_opt<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        return Ok(None);
    }
    let s = s.replacen(",", ".", 1);
    Ok(Some(
        Decimal::from_str_exact(&s).map_err(serde::de::Error::custom)?,
    ))
}

#[allow(dead_code)]
pub fn dec_from_swe_num<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    dec_from_swe_num_opt(deserializer)
        .and_then(|r| r.ok_or(serde::de::Error::custom("Empty number")))
}
