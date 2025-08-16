use anyhow::anyhow;
use rust_decimal::{Decimal, dec};
use serde::Serialize;

use crate::dto::{Transaction, TransactionType};

pub struct CashFlow {
    r#type: CashFlowType,
    pub client: u16,
    pub tx: u32,
    amount: Decimal,
    /// Whether the transaction is under_dispute, use to check when we receive a resolve or chargeback
    pub under_dispute: bool,
}

pub enum CashFlowType {
    Deposit,
    Withdrawal,
}

impl CashFlow {
    pub fn under_dispute(&mut self, value: bool) {
        self.under_dispute = value
    }
}

impl TryFrom<&Transaction> for CashFlow {
    type Error = anyhow::Error;

    fn try_from(value: &Transaction) -> anyhow::Result<Self> {
        match value.amount {
            Some(v) if v >= dec!(0) => {
                let cash_flow_type = match value.r#type {
                    TransactionType::Deposit => CashFlowType::Deposit,
                    TransactionType::Withdrawal => CashFlowType::Withdrawal,
                    _ => {
                        log::debug!("trying to convert an unsupported transaction to a cash flow");
                        return Err(anyhow!(
                            "a generic error occurred", // This is an internal error related to
                                                        // wrong usage of the method, we don't want to expose these details to the
                                                        // frontend
                        ));
                    }
                };
                Ok(CashFlow {
                    r#type: cash_flow_type,
                    client: value.client,
                    tx: value.tx,
                    amount: v,
                    under_dispute: false,
                })
            }
            Some(_) => Err(anyhow!("tx {}: has a negative value", value.tx)),
            None => Err(anyhow!("tx {}: value is missing", value.tx)),
        }
    }
}

/// A snapshot of clients' accounts after processing the transactions
#[derive(Debug, Default, PartialEq, Eq, Hash, Serialize)] //TODO: remove ser
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

    pub fn deposit(&mut self, cf: &CashFlow) -> anyhow::Result<()> {
        match cf.r#type {
            CashFlowType::Deposit => {
                let amount = cf.amount;
                self.available += amount;
                self.total = self.available + self.held;
                Ok(())
            }
            _ => {
                log::debug!("performing a deposit with a cash flow of wrong type");
                Err(anyhow!("a generic error occurred"))
            }
        }
    }

    pub fn withdraw(&mut self, cf: &CashFlow) {
        // We are assuming that this should not block the operations, a customer that requires more
        // than the available results in ignoring the operation and logging the error
        let amount = cf.amount;
        if amount <= self.available {
            self.available -= amount;
            self.total = self.available + self.held;
        } else {
            log::error!(
                "user {} does not have enough money to perform a withdraw",
                self.client
            )
        }
    }

    pub fn dispute(&mut self, cf: &CashFlow) {
        let amount = cf.amount;
        self.available -= amount;
        self.held += amount;
        //total remains the same as we are only moving from available to held
    }

    pub fn resolve(&mut self, cf: &CashFlow) {
        let amount = cf.amount;
        self.held -= amount;
        self.available += amount;
    }

    /// A chargeback related to a transaction, if this occurs the account will be locked
    /// preventing user to perform additional operations
    pub fn chargeback(&mut self, cf: &CashFlow) {
        let amount = cf.amount; // We are assuming that a dispute can lead to a negative balance (e.g., due to a subsequent
        // withdrawal), therefore we lock the account for the investigations
        self.locked = true;
        self.held -= amount;
        self.total -= amount;
    }
}
