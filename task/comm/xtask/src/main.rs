use xtask_util::get_cwd_repo_path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(flatten)]
    Base(xtask_base::Command),

    /// Benchmark your implementation against C++ code.
    Bench,
}

////////////////////////////////////////////////////////////////////////////////

fn bench() -> Result<()> {
    let repo_path = get_cwd_repo_path()?;
    let sh = Shell::new().context("failed to create shell")?;

    cmd!(sh, "cargo build --release").run()?;
    cmd!(
        sh,
        "c++ src/main.cpp -o {repo_path}/target/release/comm_cpp -O3 -std=c++11"
    )
    .run()?;
    cmd!(sh, "cargo bench").run()?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Base(cmd) => xtask_base::run_command(cmd),
        Command::Bench => bench(),
    }
}
