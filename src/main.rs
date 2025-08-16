use std::process::exit;

use crate::engine::parser;

use clap::Parser;

mod dto;
mod engine;
mod validator;

#[derive(Parser)]
struct Cli {
    path: String, //TODO: check valid path
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    if let Err(err) = parser(args.path) {
        print!("An error occurred during the processing: {}", err);
        exit(1)
    }
}
