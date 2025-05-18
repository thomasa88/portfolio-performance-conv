use rust_decimal::{Decimal, dec, prelude::Zero};
use serde::Deserialize;

use crate::pp;
use crate::types::{Currency, dec_from_swe_num_opt};
use crate::yahoo_symbol;

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

#[derive(Debug, Deserialize, PartialEq)]
enum AvanzaType {
    Köp,
    Sälj,
    Värdepappersöverföring,
    Ränta,
    Insättning,
    Uttag,
    Övrigt,
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
                let exch: Option<Decimal> = line.valutakurs.map(|v| (dec!(1.0) / v).round_dp(4));
                Some(pp::Transaction::Portfolio(pp::PortfolioTransaction {
                    date: line.datum,
                    securities_account: Some(line.konto.clone()),
                    cash_account: Some(line.konto),
                    type_: if line.typ_av_transaktion == AvanzaType::Köp {
                        pp::PortfolioType::Buy
                    } else {
                        pp::PortfolioType::Sell
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
                    security_name,
                    note: None,
                }))
            }
            AvanzaType::Värdepappersöverföring => {
                let type_ = if line.antal.as_ref().unwrap().is_sign_negative() {
                    pp::PortfolioType::DeliveryOutbound
                } else {
                    pp::PortfolioType::DeliveryInbound
                };
                Some(pp::Transaction::Portfolio(pp::PortfolioTransaction {
                    date: line.datum.clone(),
                    securities_account: Some(line.konto),
                    cash_account: None,
                    type_,
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
                    security_name,
                    note: line.vardepapper_beskrivning,
                }))
            }
            AvanzaType::Övrigt if line.antal.is_some() => {
                // Could be sell/move of defaulted stocks.
                let type_ = if line.antal.as_ref().unwrap().is_sign_negative() {
                    pp::PortfolioType::DeliveryOutbound
                } else {
                    pp::PortfolioType::DeliveryInbound
                };
                Some(pp::Transaction::Portfolio(pp::PortfolioTransaction {
                    date: line.datum.clone(),
                    securities_account: Some(line.konto),
                    cash_account: None,
                    type_,
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
                    security_name,
                    note: line.vardepapper_beskrivning,
                }))
            }
            AvanzaType::Övrigt if line.antal.is_none() => {
                // Could be transfer of money to the credit account. Could it be taxes?
                Some(pp::Transaction::Account(pp::AccountTransaction {
                    date: line.datum,
                    cash_account: line.konto,
                    securities_account: None,
                    type_: if line.belopp.unwrap_or_default() <= Decimal::zero() {
                        pp::AccountType::Removal
                    } else {
                        pp::AccountType::Deposit
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
                Some(pp::Transaction::Account(pp::AccountTransaction {
                    date: line.datum,
                    cash_account: line.konto,
                    securities_account: None,
                    type_: if line.typ_av_transaktion == AvanzaType::Insättning {
                        pp::AccountType::Deposit
                    } else {
                        pp::AccountType::Removal
                    },
                    value: line.belopp.unwrap(),
                    transaction_currency: line.transaktionsvaluta,
                    note: line.vardepapper_beskrivning,
                }))
            }
            AvanzaType::Ränta => Some(pp::Transaction::Account(pp::AccountTransaction {
                date: line.datum,
                cash_account: line.konto,
                securities_account: None,
                type_: if line.belopp.unwrap_or_default() <= Decimal::zero() {
                    pp::AccountType::InterestCharge
                } else {
                    pp::AccountType::Interest
                },
                value: line.belopp.unwrap(),
                transaction_currency: line.transaktionsvaluta,
                note: line.vardepapper_beskrivning,
            })),
        } {
            match pp {
                pp::Transaction::Portfolio(portfolio_transaction) => {
                    portfolio_transactions.serialize(portfolio_transaction)?;
                }
                pp::Transaction::Account(account_transaction) => {
                    account_transactions.serialize(account_transaction)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    

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
