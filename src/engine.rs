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

    // So we can validate each transaction related to deposits and withdrawals
    let cash_flows: Vec<CashFlow> = transactions
        .iter()
        .filter(|t| t.r#type == TransactionType::Deposit || t.r#type == TransactionType::Withdrawal)
        .map(CashFlow::try_from)
        .collect::<anyhow::Result<Vec<CashFlow>>>()?;

    let mut cash_flows_hm: HashMap<u32, CashFlow> =
        cash_flows.into_iter().map(|cf| (cf.tx, cf)).collect();

    for tx in transactions {
        let account = accounts
            .entry(tx.client)
            .or_insert(Account::default().client(tx.client));

        // When the account is locked, the customer cannot perform additional requests
        if account.locked {
            log::warn!("tx {}: received a request for a locked account", tx.tx);
            continue;
        }

        match tx.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                match cash_flows_hm.get(&tx.tx) {
                    Some(cf) => account.insert(cf),
                    _ => Err(anyhow!("a generic error has occurred"))?, // this should never happen
                                                                        // as we stored all the transactions into the cash flows
                }
            }
            TransactionType::Dispute => {
                // We assume that a dispute for a non-existing transaction can be ignored since is
                // an error from partner
                match cash_flows_hm.get_mut(&tx.tx) {
                    Some(cf) if cf.client == tx.client && !cf.under_dispute => {
                        account.dispute(cf);
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
                handle_dispute(&mut cash_flows_hm, tx, |cf| account.resolve(cf), "resolve")
            }
            TransactionType::Chargeback => handle_dispute(
                &mut cash_flows_hm,
                tx,
                |cf| account.chargeback(cf),
                "chargeback",
            ),
        }
    }

    Ok(accounts.into_values().map(AccountResponse::from).collect())
}

fn handle_dispute<F>(
    cash_flows_hm: &mut HashMap<u32, CashFlow>,
    tx: &Transaction,
    mut f: F,
    r#type: &str,
) where
    F: FnMut(&mut CashFlow),
{
    // We assume that if the transaction is not under dispute it is a partner error,
    // therefore we can ignore the req
    match cash_flows_hm.get_mut(&tx.tx) {
        Some(cf) if cf.client == tx.client && cf.under_dispute => {
            f(cf);
        }
        Some(_) => {
            log::warn!(
                "tx {}: received a {} request for a transaction that is not under dispute or related to wrong client",
                tx.tx,
                r#type
            )
        }
        _ => {
            log::warn!(
                "tx {}: received a {} request for a transaction that does not exist",
                tx.tx,
                r#type
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rust_decimal::{Decimal, dec};

    use crate::dto::{Transaction, TransactionType};

    use super::*;

    // helpful method to build transaction, useful also if we add additional field and we don't
    // want to break tests
    fn build_transaction(
        transaction_type: TransactionType,
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    ) -> Transaction {
        Transaction {
            r#type: transaction_type,
            client,
            tx,
            amount,
        }
    }

    fn build_account(
        client: u16,
        available: Decimal,
        held: Decimal,
        total: Decimal,
        locked: bool,
    ) -> AccountResponse {
        AccountResponse {
            client,
            available,
            held,
            total,
            locked,
        }
    }

    #[test]
    fn test_deposit_and_withdrawal() {
        let client = 1;

        let transactions = vec![
            build_transaction(TransactionType::Deposit, client, 1, Some(dec!(10))),
            build_transaction(TransactionType::Withdrawal, client, 2, Some(dec!(5))),
        ];

        let expected = vec![build_account(client, dec!(5), dec!(0), dec!(5), false)];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_multiple_accounts() {
        let client1 = 1;
        let client2 = 2;

        let transactions = vec![
            build_transaction(TransactionType::Deposit, client1, 1, Some(dec!(10))),
            build_transaction(TransactionType::Deposit, client2, 2, Some(dec!(5))),
        ];

        let accounts = vec![
            build_account(client1, dec!(10), dec!(0), dec!(10), false),
            build_account(client2, dec!(5), dec!(0), dec!(5), false),
        ];

        let processed_accounts = register_transactions_for_customers(&transactions).unwrap();

        let actual: HashSet<_> = processed_accounts.into_iter().collect();
        let expected: HashSet<_> = accounts.into_iter().collect();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_resolve_dispute() {
        let client = 1;

        let transactions = vec![
            build_transaction(TransactionType::Deposit, client, 1, Some(dec!(10))),
            build_transaction(TransactionType::Dispute, client, 1, None),
            build_transaction(TransactionType::Resolve, client, 1, None),
        ];

        let expected = vec![build_account(client, dec!(10), dec!(0), dec!(10), false)];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_handle_chargeback() {
        let client1 = 1;
        let client2 = 2;

        let transactions = vec![
            build_transaction(TransactionType::Deposit, client1, 1, Some(dec!(10))),
            build_transaction(TransactionType::Dispute, client1, 1, None),
            build_transaction(TransactionType::Chargeback, client1, 1, None),
            build_transaction(TransactionType::Deposit, client1, 2, Some(dec!(10))), // to make sure
            // a client cannot perform additional operation
            build_transaction(TransactionType::Deposit, client2, 3, Some(dec!(10))),
        ];

        let accounts = vec![
            build_account(client1, dec!(0), dec!(0), dec!(0), true),
            build_account(client2, dec!(10), dec!(0), dec!(10), false),
        ];

        let processed_accounts = register_transactions_for_customers(&transactions).unwrap();

        let actual: HashSet<_> = processed_accounts.into_iter().collect();
        let expected: HashSet<_> = accounts.into_iter().collect();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_handle_withdrawal_with_not_enough_money() {
        let client = 1;

        let transactions = vec![build_transaction(
            TransactionType::Withdrawal,
            client,
            1,
            Some(dec!(10)),
        )];

        let expected = vec![build_account(client, dec!(0), dec!(0), dec!(0), false)];

        let actual = register_transactions_for_customers(&transactions).unwrap();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_handle_dispute_for_wrong_client() {
        let client = 1;

        let transactions = vec![
            build_transaction(TransactionType::Deposit, client, 1, Some(dec!(10))),
            build_transaction(TransactionType::Dispute, client, 1, None),
            build_transaction(TransactionType::Chargeback, 2, 1, None),
        ];

        let accounts = vec![
            build_account(client, dec!(0), dec!(10), dec!(10), false),
            build_account(2, dec!(0), dec!(0), dec!(0), false),
        ];

        let processed_accounts = register_transactions_for_customers(&transactions).unwrap();

        // as we don't mind about the order of the results
        let actual: HashSet<_> = processed_accounts.into_iter().collect();
        let expected: HashSet<_> = accounts.into_iter().collect();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_negative_amount() {
        let tx1 = build_transaction(TransactionType::Deposit, 1, 1, Some(dec!(-1)));

        let transactions = vec![tx1];

        let processed_accounts = register_transactions_for_customers(&transactions);

        assert!(processed_accounts.is_err())
    }

    #[test]
    fn test_wrong_scale_amount() {
        let tx1 = build_transaction(TransactionType::Deposit, 1, 1, Some(dec!(1.12345)));

        let transactions = vec![tx1];

        let processed_accounts = register_transactions_for_customers(&transactions);

        assert!(processed_accounts.is_err())
    }

    #[test]
    fn test_missing_amount() {
        let tx1 = build_transaction(TransactionType::Deposit, 1, 1, None);

        let transactions = vec![tx1];

        let processed_accounts = register_transactions_for_customers(&transactions);

        assert!(processed_accounts.is_err())
    }
}
