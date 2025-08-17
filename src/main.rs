use std::process::exit;

use crate::engine::run;

use clap::Parser;

mod domain;
mod dto;
mod engine;
mod error;
mod validator;

#[derive(Parser)]
struct Cli {
    path: String,
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    if let Err(err) = run(args.path) {
        print!("An error occurred during the processing: {}", err);
        exit(1)
    }
}
