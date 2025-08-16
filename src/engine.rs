use std::error::Error;
use std::io;

use csv::Trim::All;
use csv::{ReaderBuilder, Writer};
use itertools::Itertools;

use crate::dto::{Account, Transaction, TransactionType};
use crate::validator::validate_transactions;

pub fn parser(path: String) -> Result<(), Box<dyn Error>> {
    let mut reader = ReaderBuilder::new()
        .trim(All) // as we want to accept csv with with whitespaces
        .from_path(path)?;

    let transactions: Vec<Transaction> = reader
        .deserialize::<Transaction>()
        .collect::<Result<Vec<_>, csv::Error>>()?;

    validate_transactions(&transactions)?;

    let grouped_transactions = transactions.into_iter().into_group_map_by(|tx| tx.client); // as
    // we want to preserve the order of the transactions

    let mut writer = Writer::from_writer(io::stdout());

    for (client, tx) in grouped_transactions {
        let account = register_transaction_for_customer(client, tx);
        writer.serialize(account?)?;
    }

    writer.flush()?;

    Ok(())
}

fn register_transaction_for_customer(
    client_id: u16,
    transactions: Vec<Transaction>,
) -> anyhow::Result<Account> {
    let mut account = Account::default().client(client_id);

    // Whether the transaction is under_dispute, use to check when we receive a resolve
    let mut under_dispute: Vec<&Transaction> = Vec::new();

    for tx in &transactions {
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

    Ok(account)
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

        let expected = Account {
            client,
            available: dec!(5),
            held: dec!(0),
            total: dec!(5),
            locked: false,
        };

        let actual = register_transaction_for_customer(client, transactions).unwrap();

        assert_eq!(actual, expected)
    }
}
