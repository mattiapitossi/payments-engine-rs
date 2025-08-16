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
    pub amount: Option<Decimal>, //TODO: check amount is positive
}

impl Transaction {
    // we are only expecting positive amounts
    fn validate(&self) -> anyhow::Result<&Transaction> {
        if let Some(amount) = self.amount {
            if amount >= dec!(0) {
                Ok(self)
            } else {
                Err(anyhow!("amount provided is negative"))
            } //TODO: add id of the tx in the error
        } else {
            Ok(self)
        }
    }

    pub fn amount(&self) -> anyhow::Result<Decimal> {
        match self.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => match self.amount {
                Some(r) => Ok(r),
                None => Err(anyhow!("is")), //TODO: provide better error explanation
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
    Chargeback,
}

/// A snapshot of clients' accounts after processing the transactions
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Account {
    //TODO: output with 4 decimal places
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
        self.available -= amount; //TODO: fail if the funds are not enough
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

    /// A chargeback related to a transaction, if this occurs the account will be locked
    /// preventing user to perform additional operations
    pub fn chargeback(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        self.locked = true;
        self.held -= transaction.amount()?;
        self.total -= transaction.amount()?;
        Ok(())
    }
}
