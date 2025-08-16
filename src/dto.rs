use anyhow::anyhow;
use rust_decimal::{Decimal, dec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub r#type: TransactionType,
    pub client: u16,
    /// The globally unique id of the transaction
    pub tx: u32,
    /// Amount is required only for deposit or a withdrawal
    pub amount: Option<Decimal>,
}

impl Transaction {
    fn get_amount_or_error(&self) -> anyhow::Result<Decimal> {
        match self.amount {
            Some(v) if v >= dec!(0) => Ok(v),
            Some(_) => Err(anyhow!("tx {}: has a negative amount", self.tx)),
            None => Err(anyhow!("tx {}: amount is not present", self.tx)),
        }
    }
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

#[derive(Serialize)]
pub struct AccountResponse {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    /// An account is locked when a charge back occurs
    pub locked: bool,
}
