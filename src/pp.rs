use rust_decimal::Decimal;
use serde::Serialize;
use crate::types::Currency;

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
