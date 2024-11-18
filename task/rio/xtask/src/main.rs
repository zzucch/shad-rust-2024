use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

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

    /// Check rio-net.
    #[clap(name = "check-net")]
    CheckNet,

    /// Submit rio-net.
    #[clap(name = "submit-net")]
    SubmitNet,
}

fn check_net() -> Result<()> {
    let sh = Shell::new()?;
    cmd!(sh, "cargo xtask check --no-default-features --features net").run()?;
    Ok(())
}

fn submit_net() -> Result<()> {
    let sh = Shell::new()?;
    cmd!(sh, "cargo xtask submit --subtask net").run()?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Base(cmd) => xtask_base::run_command(cmd),
        Command::CheckNet => check_net(),
        Command::SubmitNet => submit_net(),
    }
}
