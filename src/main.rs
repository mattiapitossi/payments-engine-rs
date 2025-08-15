use std::process::exit;

use crate::engine::parser;

use clap::Parser;

mod dto;
mod engine;

#[derive(Parser)]
struct Cli {
    path: String,
}

fn main() {
    let args = Cli::parse();

    if let Err(err) = parser(args.path) {
        print!("err {}", err);
        exit(1)
    }
}
