use rust_decimal::{Decimal, dec};
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::pp;
use crate::types::{Currency, dec_from_swe_num_opt};
use crate::{ProgressSender, yahoo_symbol};

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

pub async fn convert(
    input: &std::path::Path,
    writer: &mut pp::CsvWriter,
    mut progress: ProgressSender,
) -> anyhow::Result<()> {
    let yahoo = yahoo_symbol::Yahoo::new_with_progress(progress.clone());
    let mut read = File::open(&input).await.map(BufReader::new)?;
    let mut line_buf = String::new();
    let mut num_lines = 0;
    while read.read_line(&mut line_buf).await? != 0 {
        line_buf.clear();
        num_lines += 1;
    }
    drop(line_buf);
    // Assuming one header line
    num_lines -= 1;
    progress.total(num_lines).await;
    progress.count(0).await;
    let mut reader = csv::ReaderBuilder::new().delimiter(b';').from_path(input)?;
    let mut read_records = 0;
    for line in reader.deserialize() {
        let line: AvanzaTransaction = line?;

        let mut security_name = line.vardepapper_beskrivning.clone();
        let mut y_symbol = None;
        if let Some(isin) = &line.isin {
            let y_securities = yahoo.isin_to_symbols(isin).await?;
            let security = y_securities
                .iter()
                .find(|s| s.exchange == "STO")
                .or(y_securities.first());
            if let Some(security) = security {
                y_symbol = Some(security.symbol.clone());
                if security_name.is_none() {
                    security_name = Some(security.name.clone());
                }
            }
        }
        let avanza_account = prefix_account(&line.konto);
        let transaction = match line.typ_av_transaktion {
            AvanzaType::Köp | AvanzaType::Sälj => {
                let exch: Option<Decimal> = line.valutakurs.map(|v| (dec!(1.0) / v).round_dp(4));
                Some(pp::Transaction::Portfolio(pp::PortfolioTransaction {
                    date: line.datum,
                    securities_account: Some(avanza_account.clone()),
                    cash_account: Some(avanza_account),
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
                    securities_account: Some(avanza_account),
                    cash_account: None,
                    type_,
                    value: if let (Some(antal), Some(kurs)) = (line.antal, line.kurs) {
                        antal * kurs
                    } else {
                        progress
                            .log(format!(
                                "Antar att beloppet är 0 för {} {}",
                                &line.datum,
                                &line.vardepapper_beskrivning.clone().unwrap_or_default()
                            ))
                            .await;
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
                    securities_account: Some(avanza_account),
                    cash_account: None,
                    type_,
                    value: if let (Some(antal), Some(kurs)) = (line.antal, line.kurs) {
                        antal * kurs
                    } else {
                        progress
                            .log(format!(
                                "Antar att beloppet är 0 för {} {}",
                                &line.datum,
                                &line.vardepapper_beskrivning.clone().unwrap_or_default()
                            ))
                            .await;
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
                    cash_account: avanza_account,
                    securities_account: None,
                    type_: if line.belopp.unwrap_or_default().is_sign_negative() {
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
                    cash_account: avanza_account,
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
                cash_account: avanza_account,
                securities_account: None,
                type_: if line.belopp.unwrap_or_default().is_sign_negative() {
                    pp::AccountType::InterestCharge
                } else {
                    pp::AccountType::Interest
                },
                value: line.belopp.unwrap(),
                transaction_currency: line.transaktionsvaluta,
                note: line.vardepapper_beskrivning,
            })),
        };
        if let Some(t) = transaction {
            writer.write(&t)?;
        }

        read_records += 1;
        progress.count(read_records).await;
    }

    // Correct the total, now that we now the number of records, in case the original
    // guess was wrong.
    progress.total(read_records).await;

    yahoo.save_cache().await;
    Ok(())
}

fn prefix_account(account_name: &str) -> String {
    format!("Avanza {account_name}")
}
