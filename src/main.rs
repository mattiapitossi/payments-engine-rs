use std::process::exit;

use crate::engine::parser;

mod domain;
mod engine;

fn main() {
    if let Err(err) = parser() {
        print!("err {}", err);
        exit(1)
    }
}
