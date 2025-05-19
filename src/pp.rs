use std::{collections::HashSet, fs::File, path::Path};

use crate::types::Currency;
use rust_decimal::Decimal;
use serde::Serialize;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Transaction {
    Portfolio(PortfolioTransaction),
    Account(AccountTransaction),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PortfolioTransaction {
    pub date: String,
    #[serde(rename = "Securities Account")]
    pub securities_account: Option<String>,
    #[serde(rename = "Cash Account")]
    pub cash_account: Option<String>,
    #[serde(rename = "Type")]
    pub type_: PortfolioType,
    pub value: Decimal,
    #[serde(rename = "Transaction Currency")]
    pub transaction_currency: Currency,
    #[serde(rename = "Gross Amount")]
    pub gross_amount: Option<Decimal>,
    // This is the currency of the gross amount
    #[serde(rename = "Currency Gross Amount")]
    pub currency_gross_amount: Option<Currency>,
    #[serde(rename = "Exchange Rate")]
    pub exchange_rate: Option<Decimal>,
    pub fees: Option<Decimal>,
    pub taxes: Option<Decimal>,
    pub shares: Option<Decimal>,
    #[serde(rename = "ISIN")]
    pub isin: Option<String>,
    #[serde(rename = "WKN")]
    pub wkn: Option<String>,
    #[serde(rename = "Ticker Symbol")]
    pub ticker_symbol: Option<String>,
    #[serde(rename = "Security Name")]
    pub security_name: Option<String>,
    pub note: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum PortfolioType {
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
pub struct AccountTransaction {
    pub date: String,
    #[serde(rename = "Cash Account")]
    pub cash_account: String,
    #[serde(rename = "Securities Account")]
    pub securities_account: Option<String>,
    #[serde(rename = "Type")]
    pub type_: AccountType,
    pub value: Decimal,
    #[serde(rename = "Transaction Currency")]
    pub transaction_currency: Currency,
    // #[serde(rename = "Gross Amount")]
    // pub gross_amount: Option<Decimal>,
    // This is the currency of the gross amount
    // #[serde(rename = "Currency Gross Amount")]
    // pub currency_gross_amount: Option<Currency>,
    // #[serde(rename = "Exchange Rate")]
    // pub exchange_rate: Option<Decimal>,
    // pub fees: Option<Decimal>,
    // pub taxes: Option<Decimal>,
    // pub shares: Option<Decimal>,
    // #[serde(rename = "ISIN")]
    // pub isin: Option<String>,
    // #[serde(rename = "WKN")]
    // pub wkn: Option<String>,
    // #[serde(rename = "Ticker Symbol")]
    // pub ticker_symbol: Option<String>,
    // #[serde(rename = "Security Name")]
    // pub security_name: Option<String>,
    pub note: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum AccountType {
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

#[derive(Debug, thiserror::Error)]
pub enum CsvWriterError {
    #[error("Failed to create file")]
    CreateFileFailed(#[from] csv::Error),
    #[error("Failed to write to file")]
    WriteFailed,
}

pub struct CsvWriter {
    portfolio_trans: csv::Writer<File>,
    account_trans: csv::Writer<File>,
    security_accounts: HashSet<String>,
    cash_accounts: HashSet<String>,
}

impl CsvWriter {
    /// Creates a new CSV writer for the given portfolio and account paths.
    /// The portfolio path is for the securities account transactions,
    /// and the account path is for the savings account transactions.
    /// The CSV files will be created if they do not exist.
    /// If the files already exist, they will be overwritten.
    pub fn new<T: AsRef<Path>>(portfolio_path: T, account_path: T) -> Result<Self, CsvWriterError> {
        Ok(CsvWriter {
            portfolio_trans: csv::WriterBuilder::new()
                .delimiter(b',')
                .from_path(portfolio_path)
                .map_err(CsvWriterError::CreateFileFailed)?,
            account_trans: csv::WriterBuilder::new()
                .delimiter(b',')
                .from_path(account_path)
                .map_err(CsvWriterError::CreateFileFailed)?,
            security_accounts: HashSet::new(),
            cash_accounts: HashSet::new(),
        })
    }

    pub fn write(&mut self, transaction: &Transaction) -> Result<(), CsvWriterError> {
        match transaction {
            Transaction::Portfolio(t) => {
                self.portfolio_trans
                    .serialize(t)
                    .map_err(|_| CsvWriterError::WriteFailed)?;
                if let Some(security_account) = &t.securities_account {
                    self.security_accounts.insert(security_account.clone());
                }
                if let Some(cash_account) = &t.cash_account {
                    self.cash_accounts.insert(cash_account.clone());
                }
            }
            Transaction::Account(t) => {
                self.account_trans
                    .serialize(t)
                    .map_err(|_| CsvWriterError::WriteFailed)?;
                {
                    if let Some(security_account) = &t.securities_account {
                        self.security_accounts.insert(security_account.clone());
                    }
                    self.cash_accounts.insert(t.cash_account.clone());
                }
            }
        };
        Ok(())
    }

    /// Returns an accumulated list of all created accounts.
    pub fn security_accounts(&self) -> &HashSet<String> {
        &self.security_accounts
    }

    /// Returns an accumulated list of all created accounts.
    pub fn cash_accounts(&self) -> &HashSet<String> {
        &self.cash_accounts
    }
}
