use anyhow::{Context, anyhow};
use std::collections::HashMap;
use std::io;

use csv::Trim::All;
use csv::{ReaderBuilder, Writer};

use crate::domain::{Account, CashFlow};
use crate::dto::{AccountResponse, Transaction, TransactionType};
use crate::validator::validate_transactions;

pub fn run(path: &str) -> anyhow::Result<()> {
    let mut reader = ReaderBuilder::new()
        .trim(All) // as we want to accept CSV with with whitespaces
        .from_path(path)
        .with_context(|| format!("cannot find path {}", path))?;

    let transactions: Vec<Transaction> = reader
        .deserialize::<Transaction>()
        .collect::<Result<Vec<_>, csv::Error>>()?;

    // Before performing a processing we performe a validation against all transactions
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
) -> anyhow::Result<Vec<AccountResponse>> {
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    let mut cash_flows: HashMap<u32, CashFlow> = transactions
        .iter()
        .filter_map(|t| CashFlow::try_from(t).ok())
        .map(|cf| (cf.tx, cf))
        .collect();

    for tx in transactions {
        let account = accounts
            .entry(tx.client)
            .or_insert(Account::default().client(tx.client));

        // When the account is locked, the customer cannot perform additional requests
        if account.locked {
            break;
        }

        match tx.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                match cash_flows.get(&tx.tx) {
                    Some(cf) => account.insert(cf),
                    _ => Err(anyhow!("a generic error has occurred"))?, // this should never happen
                                                                        // as we stored all the transactions into the cash flows
                }
            }
            TransactionType::Dispute => {
                // We assume that a dispute for a non-existing transaction can be ignored since is
                // an error from partner
                match cash_flows.get_mut(&tx.tx) {
                    Some(cf) if cf.client == tx.client && !cf.under_dispute => {
                        account.dispute(cf);
                        cf.under_dispute(true);
                    }
                    Some(cf) if cf.client == tx.client => {
                        log::warn!(
                            "tx {}: received a dispute request for a transaction that is already under dispute, discarding the request",
                            tx.tx
                        );
                    }
                    _ => {
                        log::warn!(
                            "tx {}: received a dispute for a non-existing transaction or related to wrong client",
                            tx.tx
                        )
                    }
                }
            }
            TransactionType::Resolve => {
                // We assume that if the transaction is not under dispute it is a partner error,
                // therefore we can ignore the resolve req
                match cash_flows.get_mut(&tx.tx) {
                    Some(cf) if cf.client == tx.client && cf.under_dispute => {
                        account.resolve(cf);
                        cf.under_dispute(false);
                    }
                    _ => {
                        log::warn!(
                            "tx {}: received a resolve request for a transaction that is not under dispute or related to wrong client",
                            tx.tx
                        )
                    }
                }
            }
            TransactionType::Chargeback => {
                // We assume that if the transaction is not under dispute it is a partner error,
                // therefore we can ignore the Chargeback req
                match cash_flows.get_mut(&tx.tx) {
                    Some(cf) if cf.client == tx.client && cf.under_dispute => {
                        account.chargeback(cf);
                        cf.under_dispute(false);
                    }
                    _ => {
                        log::warn!(
                            "tx {}: received a chargeback request for a transaction that is not under dispute or related to wrong client",
                            tx.tx
                        )
                    }
                }
            }
        }
    }

    Ok(accounts.into_values().map(AccountResponse::from).collect())
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

        let expected = vec![AccountResponse {
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
            AccountResponse {
                client: client1,
                available: dec!(10),
                held: dec!(0),
                total: dec!(10),
                locked: false,
            },
            AccountResponse {
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

        let expected = vec![AccountResponse {
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

        let expected = vec![AccountResponse {
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

        let expected = vec![AccountResponse {
            client,
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: false,
        }];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_handle_dispute_for_wrong_client() {
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

        // Receiving a chargeback for right transaction but wrong client
        let tx3 = Transaction {
            r#type: TransactionType::Chargeback,
            client: 2,
            tx: 1,
            amount: None,
        };

        let transactions = vec![tx1, tx2, tx3];

        let accounts = vec![
            AccountResponse {
                client,
                available: dec!(0),
                held: dec!(10),
                total: dec!(10),
                locked: false,
            },
            AccountResponse {
                client: 2,
                available: dec!(0),
                held: dec!(0),
                total: dec!(0),
                locked: false,
            },
        ];

        let processed_accounts = register_transactions_for_customers(&transactions).unwrap();

        // as we don't mind about the order of the resuls
        let actual: HashSet<_> = processed_accounts.into_iter().collect();
        let expected: HashSet<_> = accounts.into_iter().collect();

        assert_eq!(actual, expected)
    }
}
