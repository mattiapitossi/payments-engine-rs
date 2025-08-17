use std::collections::HashSet;

use anyhow::anyhow;

use crate::dto::{Transaction, TransactionType};

pub fn validate_transactions(transactions: &[Transaction]) -> anyhow::Result<()> {
    let deposits_and_withdrawals: Vec<&Transaction> = transactions
        .iter()
        .filter(|t| t.r#type == TransactionType::Deposit || t.r#type == TransactionType::Withdrawal)
        .collect();

    let unique_ids = deposits_and_withdrawals
        .iter()
        .map(|t| t.tx)
        .collect::<HashSet<u32>>();

    // Transaction ids must be unique if it's a deposit or a withdrawal
    if unique_ids.len() != deposits_and_withdrawals.len() {
        return Err(anyhow!("Transaction ids are not unique!"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use rust_decimal::dec;

    use crate::{
        dto::{Transaction, TransactionType},
        validator::validate_transactions,
    };

    #[test]
    fn test_transactions_have_unique_id() {
        let tx1 = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: Some(dec!(10)),
        };

        let tx3 = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 2,
            amount: Some(dec!(10)),
        };

        let transactions = vec![tx1, tx2, tx3];

        assert!(validate_transactions(&transactions).is_ok())
    }

    #[test]
    fn test_transactions_does_not_have_unique_id() {
        let tx1 = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx3 = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let transactions = vec![tx1, tx2, tx3];

        assert!(validate_transactions(&transactions).is_err())
    }
}
