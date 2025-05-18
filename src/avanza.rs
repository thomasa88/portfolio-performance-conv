use std::ops::Deref;

use rust_decimal::{Decimal, dec};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::yahoo_symbol;

// #[derive(Debug, Deserialize)]
// struct SweNum(String);

// #[derive(Debug, Serialize)]
// struct IntNum(String);

// impl From<SweNum> for IntNum {
//     fn from(value: SweNum) -> Self {
//         IntNum(value.0.replacen(",", ".", 1))
//     }
// }

// impl TryFrom<SweNum> for Decimal {
//     type Error = rust_decimal::Error;
//     fn try_from(value: SweNum) -> Result<Self, Self::Error> {
//         Decimal::from_str_exact(&IntNum::from(value).0)
//     }
// }

#[derive(Debug, Serialize, Deserialize)]
struct Currency(String);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct AvanzaTransaction {
    datum: String,
    konto: String,
    #[serde(rename = "Typ av transaktion")]
    typ_av_transaktion: AvanzaType,
    #[serde(rename = "Värdepapper/beskrivning")]
    vardepapper_beskrivning: Option<String>,
    #[serde(deserialize_with = "dec_from_swe_num_opt")]
    antal: Option<Decimal>,
    #[serde(deserialize_with = "dec_from_swe_num_opt")]
    kurs: Option<Decimal>,
    #[serde(deserialize_with = "dec_from_swe_num_opt")]
    belopp: Option<Decimal>,
    transaktionsvaluta: Currency,
    #[serde(deserialize_with = "dec_from_swe_num_opt")]
    courtage: Option<Decimal>,
    #[serde(deserialize_with = "dec_from_swe_num_opt")]
    valutakurs: Option<Decimal>,
    instrumentvaluta: Currency,
    #[serde(rename = "ISIN")]
    isin: Option<String>,
    #[serde(deserialize_with = "dec_from_swe_num_opt")]
    resultat: Option<Decimal>,
}

#[derive(Debug)]
struct CommaDec(Decimal);

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

fn dec_from_swe_num_opt<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
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
fn dec_from_swe_num<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    dec_from_swe_num_opt(deserializer)
        .and_then(|r| r.ok_or(serde::de::Error::custom("Empty number")))
}

#[derive(Debug, Deserialize, PartialEq)]
enum AvanzaType {
    // #[serde(rename = "Köp")]
    Köp,
    // #[serde(rename = "Sälj")]
    Sälj,
    // #[serde(rename = "Värdepappersöverföring")]
    Värdepappersöverföring,
    // #[serde(rename = "Ränta")]
    Ränta,
    // #[serde(rename = "Insättning")]
    Insättning,
    // #[serde(rename = "Uttag")]
    Uttag,
    // #[serde(rename = "Övrigt")]
    Övrigt,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PortfolioTransaction {
    date: String,
    #[serde(rename = "Securities Account")]
    securities_account: Option<String>,
    #[serde(rename = "Cash Account")]
    cash_account: Option<String>,
    #[serde(rename = "Type")]
    type_: PortfolioType,
    #[serde(serialize_with = "round_dec")]
    value: Decimal,
    #[serde(rename = "Transaction Currency")]
    transaction_currency: Currency,
    #[serde(rename = "Gross Amount")]
    #[serde(serialize_with = "round_dec_opt")]
    gross_amount: Option<Decimal>,
    #[serde(rename = "Currency Gross Amount")]
    currency_gross_amount: Option<Currency>,
    #[serde(rename = "Exchange Rate")]
    #[serde(serialize_with = "round_dec_opt")]
    exchange_rate: Option<Decimal>,
    #[serde(serialize_with = "round_dec_opt")]
    fees: Option<Decimal>,
    #[serde(serialize_with = "round_dec_opt")]
    taxes: Option<Decimal>,
    #[serde(serialize_with = "round_dec_opt")]
    shares: Option<Decimal>,
    #[serde(rename = "ISIN")]
    isin: Option<String>,
    #[serde(rename = "WKN")]
    wkn: Option<String>,
    #[serde(rename = "Ticker Symbol")]
    ticker_symbol: Option<String>,
    #[serde(rename = "Security Name")]
    security_name: Option<String>,
    note: Option<String>,
}

fn round_dec_opt<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Serialize::serialize(&value.map(|v| v.round_dp(4)), serializer)
    Serialize::serialize(&value.map(|v| v), serializer)
}

fn round_dec<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // round_dp(4)
    Serialize::serialize(&value, serializer)
}

#[derive(Debug, Serialize)]
enum PortfolioType {
    Buy,
    Sell,
    #[serde(rename = "Delivery (Inbound)")]
    DeliveryInbound,
    #[serde(rename = "Delivery (Outbound)")]
    DeliveryOutbound,
    #[serde(rename = "Transfer (Inbound)")]
    TransferInbound,
    #[serde(rename = "Transfer (Outbound)")]
    TransferOutbound,
}

#[derive(Debug, Serialize)]
enum AccountTransaction {
    Interest {
        #[serde(rename = "Cash Account")]
        cash_account: String,
        date: String,
        // #[serde(rename = "Type")]
        // type_: PortfolioType,
        value: String,
        #[serde(rename = "Transaction Currency")]
        transaction_currency: String,
        #[serde(rename = "Gross Amount")]
        gross_amount: Option<String>,
        #[serde(rename = "Currency Gross Amount")]
        currency_gross_amount: Option<String>,
        #[serde(rename = "Exchange Rate")]
        exchange_rate: Option<String>,
        fees: Option<String>,
        taxes: Option<String>,
        shares: Option<String>,
        #[serde(rename = "ISIN")]
        isin: Option<String>,
        #[serde(rename = "WKN")]
        wkn: Option<String>,
        #[serde(rename = "Ticker Symbol")]
        ticker_symbol: Option<String>,
        #[serde(rename = "Security Name")]
        security_name: Option<String>,
        note: Option<String>,
    },
}

pub fn convert(input: &std::path::Path, output: &std::path::Path) -> anyhow::Result<()> {
    let yahoo = yahoo_symbol::Yahoo::new();
    let mut reader = csv::ReaderBuilder::new().delimiter(b';').from_path(input)?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_path(output)?;
    // let mut types = std::collections::HashSet::new();
    for line in reader.deserialize() {
        let line: AvanzaTransaction = line?;

        let mut security_name = line.vardepapper_beskrivning.clone();
        let mut y_symbol = None;

        let y_securities = line
            .isin
            .as_ref()
            .map(|i| yahoo.isin_to_symbols(i).unwrap());
        if let Some(ss) = y_securities {
            let security = ss.iter().find(|s| s.exchange == "STO").or(ss.first());
            if let Some(security) = security {
                y_symbol = Some(security.symbol.clone());
                if security_name.is_none() {
                    security_name = Some(security.name.clone());
                }
            }
        }
        if let Some(pp) = match line.typ_av_transaktion {
            AvanzaType::Köp | AvanzaType::Sälj => {
                let exch: Option<Decimal> = match line.valutakurs {
                    Some(v) => Some((dec!(1.0) / (Decimal::try_from(v)?)).round_dp(4)),
                    None => None,
                };
                Some(PortfolioTransaction {
                    date: line.datum,
                    securities_account: Some(line.konto.clone()),
                    cash_account: Some(line.konto),
                    type_: if line.typ_av_transaktion == AvanzaType::Köp {
                        PortfolioType::Buy
                    } else {
                        PortfolioType::Sell
                    },
                    value: -line.belopp.unwrap(),
                    transaction_currency: line.transaktionsvaluta,
                    gross_amount: None,
                    currency_gross_amount: Some(line.instrumentvaluta),
                    exchange_rate: exch,
                    fees: line.courtage,
                    taxes: None,
                    shares: line.antal.as_ref().map(Decimal::abs),
                    isin: line.isin,
                    wkn: None,
                    ticker_symbol: y_symbol,
                    security_name: security_name,
                    note: None,
                })
            }
            AvanzaType::Värdepappersöverföring => {
                let type_ = if line.antal.as_ref().unwrap().is_sign_negative() {
                    PortfolioType::DeliveryOutbound
                } else {
                    PortfolioType::DeliveryInbound
                };
                Some(PortfolioTransaction {
                    date: line.datum.clone(),
                    securities_account: Some(line.konto),
                    cash_account: None,
                    type_: type_,
                    value: if let (Some(antal), Some(kurs)) = (line.antal, line.kurs) {
                        antal * kurs
                    } else {
                        println!(
                            "Assuming value is 0 at {} {}",
                            &line.datum,
                            &line.vardepapper_beskrivning.clone().unwrap_or_default()
                        );
                        dec!(0)
                    },
                    transaction_currency: line.transaktionsvaluta,
                    gross_amount: None,
                    currency_gross_amount: Some(line.instrumentvaluta),
                    exchange_rate: None,
                    fees: line.courtage,
                    taxes: None,
                    shares: line.antal,
                    isin: line.isin,
                    wkn: None,
                    ticker_symbol: y_symbol,
                    security_name: security_name,
                    note: line.vardepapper_beskrivning,
                })
            }
            AvanzaType::Övrigt if line.antal.is_some() => {
                // Could be sell/move of defaulted stocks.
                let type_ = if line.antal.as_ref().unwrap().is_sign_negative() {
                    PortfolioType::DeliveryOutbound
                } else {
                    PortfolioType::DeliveryInbound
                };
                Some(PortfolioTransaction {
                    date: line.datum.clone(),
                    securities_account: Some(line.konto),
                    cash_account: None,
                    type_: type_,
                    value: if let (Some(antal), Some(kurs)) = (line.antal, line.kurs) {
                        antal * kurs
                    } else {
                        println!(
                            "Assuming value is 0 at {} {}",
                            &line.datum,
                            &line.vardepapper_beskrivning.clone().unwrap_or_default()
                        );
                        dec!(0)
                    },
                    transaction_currency: line.transaktionsvaluta,
                    gross_amount: None,
                    currency_gross_amount: Some(line.instrumentvaluta),
                    exchange_rate: None,
                    fees: line.courtage,
                    taxes: None,
                    shares: line.antal,
                    isin: line.isin,
                    wkn: None,
                    ticker_symbol: y_symbol,
                    security_name: security_name,
                    note: line.vardepapper_beskrivning,
                })
            }
            AvanzaType::Övrigt if line.antal.is_none() => {
                // Could be taxes or transfer of money to credit account.
                None
            }
            AvanzaType::Övrigt => {
                panic!("Should not get here");
            }
            AvanzaType::Ränta => None,
            AvanzaType::Insättning => None,
            AvanzaType::Uttag => None,
        } {
            writer.serialize(pp)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avanza() {

        // Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage;Valutakurs;Instrumentvaluta;ISIN;Resultat
        // 2021-03-15;Konto A kreditkonto;Ränta;;;;-123,45;SEK;;;;;
        // 2021-03-12;Konto A kreditkonto;Uttag;Överföring till ISK/KF-depån;;;-567,89;SEK;;;;;
        // 2022-03-12;Konto A;Insättning;Överföring från Kreditdepån;;;567,89;SEK;;;;;
        // 2022-03-12;Konto A;Köp;Global Fund;2,345;150,75;-353,45;SEK;0;;SEK;SE0012345678;
        // 2025-03-10;Konto A kreditkonto;Uttag;Överföring till ISK/KF-depån;;;-987,65;SEK;;;;;
        // 2025-03-10;Konto A;Insättning;Överföring från Kreditdepån;;;987,65;SEK;;;;;
        // 2025-03-10;Konto A;Köp;Emerging Markets;5,678;200,50;-1137,89;SEK;0;;SEK;SE0009876543;
        // 2025-03-10;Konto A;Köp;Global Index Fund;4,321;175,25;-756,45;SEK;0;;SEK;SE0018765432;
        // 2025-03-10;Konto A;Köp;Nordic Small Cap;3,210;190,10;-610,32;SEK;0;;SEK;SE0017654321;
        // 2025-03-05;Konto A kreditkonto;Uttag;Överföring till ISK/KF-depån;;;-456,78;SEK;;;;;
        // 2025-03-05;Konto A;Insättning;Överföring från Kreditdepån;;;456,78;SEK;;;;;
    }
}
