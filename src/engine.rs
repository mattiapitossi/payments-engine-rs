use anyhow::{Context, anyhow};
use std::collections::HashMap;
use std::io;

use csv::Trim::All;
use csv::{ReaderBuilder, Writer};

use crate::domain::{Account, CashFlow};
use crate::dto::{AccountResponse, Transaction, TransactionType};

pub fn run(path: &str) -> anyhow::Result<()> {
    let mut reader = ReaderBuilder::new()
        .trim(All) // as we want to accept CSV with with whitespaces
        .from_path(path)
        .with_context(|| format!("cannot find path {}", path))?;

    let mut writer = Writer::from_writer(io::stdout());

    let mut accounts: HashMap<u16, Account> = HashMap::new();
    let mut cash_flows: HashMap<u32, CashFlow> = HashMap::new();

    for transaction in reader.deserialize() {
        let record: Transaction = transaction?;
        handle_transaction(&record, &mut accounts, &mut cash_flows)?
    }

    for account in accounts
        .into_values()
        .map(AccountResponse::from)
        .collect::<Vec<AccountResponse>>()
    {
        writer.serialize(account)?;
    }

    writer.flush()?;

    Ok(())
}

fn handle_transaction(
    transaction: &Transaction,
    accounts: &mut HashMap<u16, Account>,
    cash_flows: &mut HashMap<u32, CashFlow>,
) -> anyhow::Result<()> {
    let account = accounts
        .entry(transaction.client)
        .or_insert(Account::default().client(transaction.client));

    // When the account is locked, the customer cannot perform additional requests
    if account.locked {
        log::warn!(
            "tx {}: received a request for a locked account",
            transaction.tx
        );
    } else {
        // we only store transactions that are a deposit or a withdrawal to not load every entry
        if transaction.r#type == TransactionType::Deposit
            || transaction.r#type == TransactionType::Withdrawal
        {
            let cf = CashFlow::try_from(transaction)?;
            cash_flows.insert(transaction.tx, cf);
        }

        register_transactions_for_customers(account, cash_flows, transaction)?;
    };

    Ok(())
}

fn register_transactions_for_customers(
    account: &mut Account,
    cash_flows: &mut HashMap<u32, CashFlow>,
    tx: &Transaction,
) -> anyhow::Result<()> {
    match tx.r#type {
        TransactionType::Deposit | TransactionType::Withdrawal => {
            match cash_flows.get(&tx.tx) {
                Some(cf) => account.insert(cf),
                _ => Err(anyhow!("a generic error has occurred"))?, // this should never happen
                                                                    // as we stored a deposit of withdrawal first into the cash flows
            }
        }
        TransactionType::Dispute => {
            // We assume that a dispute for a non-existing transaction can be ignored since is
            // an error from partner
            match cash_flows.get_mut(&tx.tx) {
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
            handle_dispute(cash_flows, tx, |cf| account.resolve(cf), "resolve")
        }
        TransactionType::Chargeback => {
            handle_dispute(cash_flows, tx, |cf| account.chargeback(cf), "chargeback")
        }
    }

    Ok(())
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
    ) -> Account {
        Account {
            client,
            available,
            held,
            total,
            locked,
        }
    }

    #[test]
    fn test_deposit() {
        let client = 1;

        let transaction = build_transaction(TransactionType::Deposit, client, 1, Some(dec!(10)));
        let cf = CashFlow::try_from(&transaction).unwrap();

        let mut cash_flows = HashMap::from([(cf.tx, cf)]);

        let mut account = build_account(client, dec!(5), dec!(0), dec!(5), false);

        register_transactions_for_customers(&mut account, &mut cash_flows, &transaction).unwrap();

        let account_updated = build_account(client, dec!(15), dec!(0), dec!(15), false);

        assert_eq!(account, account_updated)
    }

    //TODO: add more tests
}
