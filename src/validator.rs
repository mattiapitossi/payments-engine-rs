use std::collections::HashSet;

use anyhow::anyhow;

use crate::dto::{Transaction, TransactionType};

/// Validates transactions by checking if ids are not unique
pub fn validate_transactions(transactions: &[Transaction]) -> anyhow::Result<()> {
    let filtered_transactions: Vec<&Transaction> = transactions
        .iter()
        .filter(|t| t.r#type == TransactionType::Deposit || t.r#type == TransactionType::Withdrawal) // as the other transactions refer to an existing one
        .collect();

    let unique_ids = filtered_transactions
        .iter()
        .map(|t| t.tx)
        .collect::<HashSet<u32>>();

    if unique_ids.len() == filtered_transactions.len() {
        Ok(())
    } else {
        Err(anyhow!("Transaction ids are not unique!"))
    }
}
