use std::{error::Error, io};

use csv::{ReaderBuilder, Trim::All};

use crate::domain::Transaction;

mod domain;

fn main() -> Result<(), Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new()
        .trim(All) // as we want to accept csv with with whitespaces
        .from_reader(io::stdin());

    for result in rdr.deserialize() {
        let record: Transaction = result?;
        println!("{:?}", record);
    }
    Ok(())
}
