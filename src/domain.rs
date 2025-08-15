use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Transaction {
    r#type: TransactionType,
    client: u16,
    /// The globally unique id of the transaction
    tx: u32,
    amount: Decimal,
}

/// Types of allowed Transaction
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")] // as our input csv is lowercase
enum TransactionType {
    Deposit, //TODO: check type is string
    Withdrawal,
    Dispute,
    Resolve,
    Cargeback,
}
