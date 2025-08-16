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
            None => Err(anyhow!("tx {}: is not present", self.tx)),
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

/// A snapshot of clients' accounts after processing the transactions
#[derive(Debug, Default, PartialEq, Serialize, Eq, Hash)]
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

    pub fn deposit(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        let amount = transaction.get_amount_or_error()?;
        self.available += amount;
        self.total = self.available + self.held;
        Ok(())
    }

    pub fn withdraw(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        // We are assuming that this should not block the operations, a customer that requires more
        // than the available results in ignoring the operation and logging the error
        let amount = transaction.get_amount_or_error()?;
        if amount <= self.available {
            self.available -= amount;
            self.total = self.available + self.held;
        } else {
            log::error!(
                "user {} does not have enough money to perform a withdraw",
                self.client
            )
        }
        Ok(())
    }

    pub fn dispute(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        let amount = transaction.get_amount_or_error()?;
        self.available -= amount;
        self.held += amount;
        //total remains the same as we are only moving from available to held
        Ok(())
    }

    pub fn resolve(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        let amount = transaction.get_amount_or_error()?;
        self.held -= amount;
        self.available += amount;
        Ok(())
    }

    /// A chargeback related to a transaction, if this occurs the account will be locked
    /// preventing user to perform additional operations
    pub fn chargeback(&mut self, transaction: &Transaction) -> anyhow::Result<()> {
        let amount = transaction.get_amount_or_error()?;
        self.locked = true;
        self.held -= amount;
        self.total -= amount;
        Ok(())
    }
}
