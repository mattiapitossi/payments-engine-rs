use std::collections::HashSet;

use anyhow::anyhow;
use rust_decimal::dec;

use crate::dto::{Transaction, TransactionType};

/// Validates transactions by checking if ids are not unique
pub fn validate_transactions(transactions: &[Transaction]) -> anyhow::Result<()> {
    let deposits_and_withdrawals: Vec<&Transaction> = transactions
        .iter()
        .filter(|t| t.r#type == TransactionType::Deposit || t.r#type == TransactionType::Withdrawal) // as the other transactions refer to an existing one
        .collect();

    let unique_ids = deposits_and_withdrawals
        .iter()
        .map(|t| t.tx)
        .collect::<HashSet<u32>>();

    // Transaction ids must be unique if it's a deposit or a withdrawal
    if unique_ids.len() != deposits_and_withdrawals.len() {
        return Err(anyhow!("Transaction ids are not unique!"));
    }

    // Amount should be present if it's a deposit or a withdrawal
    if let Some(tx_missing_amount) = deposits_and_withdrawals.iter().find(|t| t.amount.is_none()) {
        return Err(anyhow!(
            "tx {}: does not have a valid amount",
            tx_missing_amount.tx
        ));
    }

    // Amount should be positive for a deposit or a withdrawal
    if let Some(tx_negative_amount) = deposits_and_withdrawals
        .iter()
        .find(|t| t.amount.unwrap() < dec!(0))
    // We can unwrap here since we already
    // performed the validation before for deposits and withdrawals
    {
        return Err(anyhow!(
            "tx {}: has a tx_negative amount",
            tx_negative_amount.tx
        ));
    }

    //TODO: validate if decimal >4

    Ok(())
}
