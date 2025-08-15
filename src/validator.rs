use std::collections::HashSet;

use anyhow::anyhow;

use crate::dto::Transaction;

/// Validates transactions by checking if ids are not unique
pub fn validate_transactions(transactions: &[Transaction]) -> anyhow::Result<()> {
    let ids = transactions.iter().map(|t| t.tx).collect::<HashSet<u32>>();

    if ids.len() == transactions.len() {
        Ok(())
    } else {
        Err(anyhow!("Transaction ids are not unique!"))
    }
}
