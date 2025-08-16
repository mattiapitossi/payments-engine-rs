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

    // Before performing any processing and create account inconsistencies, we validate the entire input file
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

    // Whether the transaction is under_dispute, use to check when we receive a resolve or
    // chargeback request
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
                account.deposit(tx)?;
            }
            TransactionType::Withdrawal => {
                account.withdraw(tx)?;
            }
            TransactionType::Dispute => {
                // We assume that a dispute for a non-existing transaction can be ignored since is
                // an error from partner
                if let Some(t) = transactions.iter().find(|t| t.tx == tx.tx) {
                    account.dispute(t)?;
                    under_dispute.push(t);
                } else {
                    log::warn!(
                        "tx {}: received a dispute for a non-existing transaction",
                        tx.tx
                    )
                }
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
                } else {
                    log::warn!(
                        "tx {}: received a resolve request for a transaction that is not under dispute",
                        tx.tx
                    )
                }
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
                } else {
                    log::warn!(
                        "tx {}: received a chargeback request for a transaction that is not under dispute",
                        tx.tx
                    )
                }
            }
        }
    }

    Ok(accounts.into_values().collect())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rust_decimal::dec;

    use crate::dto::{Transaction, TransactionType};

    use super::*;

    #[test]
    fn test_deposit_and_withdrawal() {
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

    #[test]
    fn test_multiple_accounts() {
        let client1 = 1;
        let client2 = 2;

        let tx1 = Transaction {
            r#type: TransactionType::Deposit,
            client: client1,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Deposit,
            client: client2,
            tx: 2,
            amount: Some(dec!(5)),
        };

        let transactions = vec![tx1, tx2];

        let accounts = vec![
            Account {
                client: client1,
                available: dec!(10),
                held: dec!(0),
                total: dec!(10),
                locked: false,
            },
            Account {
                client: client2,
                available: dec!(5),
                held: dec!(0),
                total: dec!(5),
                locked: false,
            },
        ];

        let processed_accounts = register_transactions_for_customers(&transactions).unwrap();

        let actual: HashSet<_> = processed_accounts.into_iter().collect();
        let expected: HashSet<_> = accounts.into_iter().collect();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_resolve_dispute() {
        let client = 1;

        let tx1 = Transaction {
            r#type: TransactionType::Deposit,
            client,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Dispute,
            client,
            tx: 1,
            amount: None,
        };

        let tx3 = Transaction {
            r#type: TransactionType::Resolve,
            client,
            tx: 1,
            amount: None,
        };

        let transactions = vec![tx1, tx2, tx3];

        let expected = vec![Account {
            client,
            available: dec!(10),
            held: dec!(0),
            total: dec!(10),
            locked: false,
        }];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_handle_chargeback() {
        let client = 1;

        let tx1 = Transaction {
            r#type: TransactionType::Deposit,
            client,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Dispute,
            client,
            tx: 1,
            amount: None,
        };

        let tx3 = Transaction {
            r#type: TransactionType::Chargeback,
            client,
            tx: 1,
            amount: None,
        };

        let transactions = vec![tx1, tx2, tx3];

        let expected = vec![Account {
            client,
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: true,
        }];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_handle_withdrawal_with_not_enough_money() {
        let client = 1;

        let tx1 = Transaction {
            r#type: TransactionType::Withdrawal,
            client,
            tx: 1,
            amount: Some(dec!(10)),
        };

        let transactions = vec![tx1];

        let expected = vec![Account {
            client,
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: false,
        }];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }
}
