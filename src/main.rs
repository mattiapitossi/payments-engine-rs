use crate::engine::run;

use clap::Parser;

mod domain;
mod dto;
mod engine;

#[derive(Parser)]
struct Cli {
    path: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Cli::parse();

    run(&args.path)
}
