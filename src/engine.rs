use std::collections::HashMap;
use std::error::Error;
use std::io;

use csv::Trim::All;
use csv::{ReaderBuilder, Writer};

use crate::dto::{Account, Transaction, TransactionType};
use crate::validator::validate_transactions;

pub fn parser(path: String) -> Result<(), Box<dyn Error>> {
    let mut reader = ReaderBuilder::new()
        .trim(All) // as we want to accept CSV with with whitespaces
        .from_path(path)?;

    let transactions: Vec<Transaction> = reader
        .deserialize::<Transaction>()
        .collect::<Result<Vec<_>, csv::Error>>()?;

    validate_transactions(&transactions)?;

    let mut writer = Writer::from_writer(io::stdout());

    let accounts = register_transactions_for_customers(&transactions)?;

    for account in accounts {
        writer.serialize(account)?;
    }

    writer.flush()?;

    Ok(())
}

fn register_transactions_for_customers(
    transactions: &[Transaction],
) -> anyhow::Result<Vec<Account>> {
    let mut accounts: HashMap<u16, Account> = HashMap::new();

    // Whether the transaction is under_dispute, use to check when we receive a resolve
    let mut under_dispute: Vec<&Transaction> = Vec::new();

    for tx in transactions {
        let account = accounts
            .entry(tx.client)
            .or_insert(Account::default().client(tx.client));

        // When the account is locked, the customer cannot perform additional requests
        if account.locked {
            break;
        }

        match tx.r#type {
            TransactionType::Deposit => {
                account.deposit(tx.amount()?);
            }
            TransactionType::Withdrawal => {
                account.withdraw(tx.amount()?);
            }
            TransactionType::Dispute => {
                // We assume that a dispute for a non-existing transaction can be ignored since is
                // an error from partner
                if let Some(t) = transactions.iter().find(|t| t.tx == tx.tx) {
                    account.dispute(t)?;
                    under_dispute.push(t);
                }
                //TODO: add logging
            }
            TransactionType::Resolve => {
                // We assume that if the transaction is not under dispute it is a partner error,
                // therefore we can ignore the resolve req
                if let Some((idx, t)) = under_dispute
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.tx == tx.tx)
                {
                    account.resolve(t)?;
                    under_dispute.remove(idx);
                }
                //TODO: add logging
            }
            TransactionType::Chargeback => {
                // We assume that if the transaction is not under dispute it is a partner error,
                // therefore we can ignore the Chargeback req
                if let Some((idx, t)) = under_dispute
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.tx == tx.tx)
                {
                    account.chargeback(t)?;
                    under_dispute.remove(idx);
                }
                //TODO: add logging
            }
        }
    }

    Ok(accounts.into_values().collect())
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use crate::dto::{Transaction, TransactionType};

    use super::*;

    #[test]
    fn test_register_transaction_for_customer() {
        let client = 1;

        let tx1 = Transaction {
            r#type: TransactionType::Deposit,
            client,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Withdrawal,
            client,
            tx: 2,
            amount: Some(dec!(5)),
        };

        let transactions = vec![tx1, tx2];

        let expected = vec![Account {
            client,
            available: dec!(5),
            held: dec!(0),
            total: dec!(5),
            locked: false,
        }];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }
}
