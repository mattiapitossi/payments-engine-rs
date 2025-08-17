use std::collections::HashSet;

use anyhow::anyhow;
use rust_decimal::dec;

use crate::domain::CashFlow;

/// Validates transactions by checking if ids are not unique
pub fn validate_transactions(cash_flows: Vec<&CashFlow>) -> anyhow::Result<()> {
    let unique_ids = cash_flows.iter().map(|t| t.tx).collect::<HashSet<u32>>();

    // Transaction ids must be unique if it's a deposit or a withdrawal
    if unique_ids.len() != cash_flows.len() {
        return Err(anyhow!("Transaction ids are not unique!"));
    }

    // Amount should be positive for a deposit or a withdrawal
    if let Some(tx_negative_amount) = cash_flows.iter().find(|t| t.amount < dec!(0)) {
        return Err(anyhow!(
            "tx {}: has a tx_negative amount",
            tx_negative_amount.tx
        ));
    }

    if let Some(tx_decimal) = cash_flows.iter().find(|t| t.amount.scale() > 4) {
        return Err(anyhow!(
            "tx {}: amount has an unsupported scale",
            tx_decimal.tx
        ));
    }

    Ok(())
}
