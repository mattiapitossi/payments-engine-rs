use anyhow::anyhow;
use rust_decimal::{Decimal, dec};

use crate::dto::{Transaction, TransactionType};

pub struct CashFlow {
    pub r#type: CashFlowType,
    /// Global unique id of the client
    pub client: u16,
    /// Global unique id of the transaction
    #[allow(dead_code)]
    pub tx: u32,
    pub amount: Decimal,
    /// Whether the cash flow is under dispute, use to check if there's a dispute request when we receive a resolve or charge back
    pub under_dispute: bool,
}

pub enum CashFlowType {
    Deposit,
    Withdrawal,
}

impl TryFrom<&Transaction> for CashFlow {
    type Error = anyhow::Error;

    fn try_from(value: &Transaction) -> anyhow::Result<Self> {
        match value.amount {
            Some(v) if v >= dec!(0) && v.scale() <= 4 => {
                let cash_flow_type = match value.r#type {
                    TransactionType::Deposit => CashFlowType::Deposit,
                    TransactionType::Withdrawal => CashFlowType::Withdrawal,
                    _ => {
                        log::error!("trying to convert an unsupported transaction to a cash flow");
                        return Err(anyhow!(
                            "a generic error occurred", // This is an internal error related to
                                                        // wrong usage of the method, we don't want to expose these details to the
                                                        // clients
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
            Some(v) if v.scale() > 4 => {
                Err(anyhow!("tx {}: has a unsupported scale (>4)", value.tx))
            }
            Some(_) => Err(anyhow!("tx {}: has a negative value", value.tx)),
            None => Err(anyhow!("tx {}: value is missing", value.tx)),
        }
    }
}

/// A snapshot of clients' accounts after processing the transactions
#[derive(Default, PartialEq, Eq, Hash, Debug)]
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

    pub fn insert(&mut self, cf: &CashFlow) {
        match cf.r#type {
            CashFlowType::Deposit => {
                let amount = cf.amount;
                self.available += amount;
                self.total = self.available + self.held;
            }
            CashFlowType::Withdrawal => {
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
        }
    }

    pub fn dispute(&mut self, cf: &mut CashFlow) {
        let amount = cf.amount;
        self.available -= amount;
        self.held += amount;
        //total remains the same as we are only moving from available to held

        cf.under_dispute = true
    }

    pub fn resolve(&mut self, cf: &mut CashFlow) {
        let amount = cf.amount;
        self.held -= amount;
        self.available += amount;

        cf.under_dispute = false
    }

    /// A chargeback related to a transaction, if this occurs the account will be locked
    /// preventing user to perform additional operations
    pub fn chargeback(&mut self, cf: &mut CashFlow) {
        let amount = cf.amount; // We are assuming that a dispute can lead to a negative balance (e.g., due to a subsequent
        // withdrawal), therefore we lock the account for the investigations
        self.locked = true;
        self.held -= amount;
        self.total -= amount;

        //finally we mark the cash flow as no more under dispute
        cf.under_dispute = false
    }
}
