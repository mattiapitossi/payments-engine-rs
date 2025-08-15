use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub r#type: TransactionType,
    pub client: u16,
    /// The globally unique id of the transaction
    pub tx: u32,
    pub amount: Decimal,
}

/// Types of allowed Transaction
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")] // as our input csv is lowercase
pub enum TransactionType {
    Deposit, //TODO: check type is string
    Withdrawal,
    //TODO: implement other types
}

/// A snapshot of clients' accounts after processing the transactions
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    /// An account is locked when a charge back occurs
    pub locked: bool,
}

impl Account {
    pub fn client(mut self, client: u16) -> Account {
        self.client = client;
        self
    }

    pub fn deposit(&mut self, amount: Decimal) {
        self.available += amount;
        self.total = self.available + self.held;
    }

    pub fn withdraw(&mut self, amount: Decimal) {
        self.available -= amount;
        self.total = self.available + self.held;
    }
}
