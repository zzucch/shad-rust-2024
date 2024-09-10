use xtask::{
    check::{check, CheckArgs},
    submit::{submit, SubmitArgs},
};

use anyhow::Result;
use clap::{Parser, Subcommand};

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    Submit(SubmitArgs),
    Check(CheckArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Command::Submit(args) => submit(args),
        Command::Check(args) => check(args),
    }
}
