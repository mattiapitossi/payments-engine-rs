use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::Account;

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub r#type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

/// Types of allowed Transaction
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")] // as our input csv is lowercase
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    /// A transaction to resolve a dispute,
    /// This unlocks the helds funds and makes it available for the customer
    Resolve,
    Chargeback,
}

#[derive(Serialize, Eq, PartialEq, Debug, Hash)]
pub struct AccountResponse {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    /// An account is locked when a charge back occurs
    pub locked: bool,
}

impl From<Account> for AccountResponse {
    fn from(value: Account) -> Self {
        AccountResponse {
            client: value.client,
            available: value.available.round_dp(4), // default values, but makes it explicit that
            // we want 4 decimals
            held: value.held.round_dp(4),
            total: value.total.round_dp(4),
            locked: value.locked,
        }
    }
}
