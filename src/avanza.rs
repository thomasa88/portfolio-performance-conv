use std::ops::Deref;

use rust_decimal::{Decimal, dec, prelude::Zero};
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

enum PpTransaction {
    Portfolio(PortfolioTransaction),
    Account(AccountTransaction),
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
    // This is the currency of the gross amount
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

#[allow(dead_code)]
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
struct AccountTransaction {
    date: String,
    #[serde(rename = "Cash Account")]
    cash_account: String,
    #[serde(rename = "Securities Account")]
    securities_account: Option<String>,
    #[serde(rename = "Type")]
    type_: AccountType,
    value: Decimal,
    #[serde(rename = "Transaction Currency")]
    transaction_currency: Currency,
    // #[serde(rename = "Gross Amount")]
    // gross_amount: Option<Decimal>,
    // This is the currency of the gross amount
    // #[serde(rename = "Currency Gross Amount")]
    // currency_gross_amount: Option<Currency>,
    // #[serde(rename = "Exchange Rate")]
    // exchange_rate: Option<Decimal>,
    // fees: Option<Decimal>,
    // taxes: Option<Decimal>,
    // shares: Option<Decimal>,
    // #[serde(rename = "ISIN")]
    // isin: Option<String>,
    // #[serde(rename = "WKN")]
    // wkn: Option<String>,
    // #[serde(rename = "Ticker Symbol")]
    // ticker_symbol: Option<String>,
    // #[serde(rename = "Security Name")]
    // security_name: Option<String>,
    note: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
enum AccountType {
    Buy,
    Deposit,
    Dividend,
    Fees,
    #[serde(rename = "Fees Refund")]
    FeesRefund,
    Interest,
    #[serde(rename = "Interest Charge")]
    InterestCharge,
    Removal,
    Sell,
    #[serde(rename = "Tax Refund")]
    TaxRefund,
    Taxes,
    #[serde(rename = "Transfer (Inbound)")]
    TransferInbound,
    #[serde(rename = "Transfer (Outbound)")]
    TransferOutbound,
}

pub fn convert(input: &std::path::Path, output: &std::path::Path) -> anyhow::Result<()> {
    let yahoo = yahoo_symbol::Yahoo::new();
    let mut reader = csv::ReaderBuilder::new().delimiter(b';').from_path(input)?;
    // Handles securities accounts
    let mut portfolio_transactions = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_path(output)?;
    // Handles deposit accounts
    let mut account_transactions = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_path(output.with_extension("pp-a.csv"))?;
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
                    Some(v) => Some((dec!(1.0) / v).round_dp(4)),
                    None => None,
                };
                Some(PpTransaction::Portfolio(PortfolioTransaction {
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
                }))
            }
            AvanzaType::Värdepappersöverföring => {
                let type_ = if line.antal.as_ref().unwrap().is_sign_negative() {
                    PortfolioType::DeliveryOutbound
                } else {
                    PortfolioType::DeliveryInbound
                };
                Some(PpTransaction::Portfolio(PortfolioTransaction {
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
                }))
            }
            AvanzaType::Övrigt if line.antal.is_some() => {
                // Could be sell/move of defaulted stocks.
                let type_ = if line.antal.as_ref().unwrap().is_sign_negative() {
                    PortfolioType::DeliveryOutbound
                } else {
                    PortfolioType::DeliveryInbound
                };
                Some(PpTransaction::Portfolio(PortfolioTransaction {
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
                }))
            }
            AvanzaType::Övrigt if line.antal.is_none() => {
                // Could be transfer of money to the credit account. Could it be taxes?
                Some(PpTransaction::Account(AccountTransaction {
                    date: line.datum,
                    cash_account: line.konto,
                    securities_account: None,
                    type_: if line.belopp.unwrap_or_default() <= Decimal::zero() {
                        AccountType::Removal
                    } else {
                        AccountType::Deposit
                    },
                    value: line.belopp.unwrap(),
                    transaction_currency: line.transaktionsvaluta,
                    note: line.vardepapper_beskrivning,
                }))
            }
            AvanzaType::Övrigt => {
                panic!("Should not get here");
            }
            AvanzaType::Insättning | AvanzaType::Uttag => {
                Some(PpTransaction::Account(AccountTransaction {
                    date: line.datum,
                    cash_account: line.konto,
                    securities_account: None,
                    type_: if line.typ_av_transaktion == AvanzaType::Insättning {
                        AccountType::Deposit
                    } else {
                        AccountType::Removal
                    },
                    value: line.belopp.unwrap(),
                    transaction_currency: line.transaktionsvaluta,
                    note: line.vardepapper_beskrivning,
                }))
            }
            AvanzaType::Ränta => Some(PpTransaction::Account(AccountTransaction {
                date: line.datum,
                cash_account: line.konto,
                securities_account: None,
                type_: if line.belopp.unwrap_or_default() <= Decimal::zero() {
                    AccountType::InterestCharge
                } else {
                    AccountType::Interest
                },
                value: line.belopp.unwrap(),
                transaction_currency: line.transaktionsvaluta,
                note: line.vardepapper_beskrivning,
            })),
        } {
            match pp {
                PpTransaction::Portfolio(portfolio_transaction) => {
                    portfolio_transactions.serialize(portfolio_transaction)?;
                }
                PpTransaction::Account(account_transaction) => {
                    account_transactions.serialize(account_transaction)?;
                }
            }
        }
    }
    Ok(())
}

fn unhandled(line: &AvanzaTransaction) {
    println!(
        "Unhandled transaction: {}, {}, {:?}, {}",
        line.datum,
        line.konto,
        line.typ_av_transaktion,
        line.vardepapper_beskrivning.clone().unwrap_or_default()
    );
    // println!("{:?}", line);
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
