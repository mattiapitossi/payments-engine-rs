use anyhow::anyhow;
use rust_decimal::Decimal;
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
    pub fn amount(&self) -> anyhow::Result<Decimal> {
        match self.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => match self.amount {
                Some(r) => Ok(r),
                None => Err(anyhow!("is")),
            },
            _ => Err(anyhow!(
                "requested an amount for a type that does not have it"
            )),
        }
    }
}

/// Types of allowed Transaction
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")] // as our input csv is lowercase
pub enum TransactionType {
    Deposit, //TODO: check type is string
    Withdrawal,
    Dispute,
    /// A transaction to resolve a dispute,
    /// This unlocks the helds funds and makes it available for the customer
    Resolve,
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

    pub fn dispute(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        self.available -= transaction.amount()?;
        self.held += transaction.amount()?;
        //total remains the same as we are only moving from available to held
        Ok(())
    }

    pub fn resolve(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        self.held -= transaction.amount()?;
        self.available += transaction.amount()?;
        Ok(())
    }
}
