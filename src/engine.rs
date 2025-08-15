use std::error::Error;
use std::io;

use csv::ReaderBuilder;
use csv::Trim::All;

use crate::domain::Transaction;

pub fn parser() -> Result<(), Box<dyn Error>> {
    let mut reader = ReaderBuilder::new()
        .trim(All) // as we want to accept csv with with whitespaces
        .from_reader(io::stdin());

    for result in reader.deserialize() {
        let record: Transaction = result?;
        println!("{:?}", record);
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
