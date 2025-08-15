use std::io;
use std::{collections::HashMap, error::Error};

use csv::Trim::All;
use csv::{ReaderBuilder, Writer};
use itertools::Itertools;

use crate::dto::{Account, Transaction, TransactionType};

pub fn parser(path: String) -> Result<(), Box<dyn Error>> {
    let mut reader = ReaderBuilder::new()
        .trim(All) // as we want to accept csv with with whitespaces
        .from_path(path)?;

    let transactions: Vec<Transaction> = reader
        .deserialize::<Transaction>()
        .collect::<Result<Vec<_>, csv::Error>>()?;

    let grouped_transactions = transactions.into_iter().into_group_map_by(|tx| tx.client); // as
    // we want to preserve the order of the transactions

    let mut writer = Writer::from_writer(io::stdout());

    for (client, tx) in grouped_transactions {
        let account = register_transaction_for_customer(client, tx);
        writer.serialize(account)?;
    }

    writer.flush()?;

    Ok(())
}

fn register_transaction_for_customer(client_id: u16, transactions: Vec<Transaction>) -> Account {
    let mut account = Account::default().client(client_id);

    for tx in transactions {
        match tx.r#type {
            TransactionType::Deposit => {
                account.deposit(tx.amount);
            }
            TransactionType::Withdrawal => {
                account.withdraw(tx.amount);
            }
        }
    }

    account
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
            amount: dec!(10),
        };

        let tx2 = Transaction {
            r#type: TransactionType::Withdrawal,
            client,
            tx: 2,
            amount: dec!(5),
        };

        let transactions = vec![tx1, tx2];

        let expected = Account {
            client,
            available: dec!(5),
            held: dec!(0),
            total: dec!(5),
            locked: false,
        };

        let actual = register_transaction_for_customer(client, transactions);

        assert_eq!(actual, expected)
    }
}
