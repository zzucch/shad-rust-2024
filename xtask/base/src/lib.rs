mod check;
mod checker_config;
mod submit;

use anyhow::Result;
use clap::Subcommand;

////////////////////////////////////////////////////////////////////////////////

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Check task.
    Check(check::CheckArgs),

    /// Submit task.
    Submit(submit::SubmitArgs),
}

pub fn run_command(cmd: Command) -> Result<()> {
    match cmd {
        Command::Check(args) => check::check(args),
        Command::Submit(args) => submit::submit(args),
    }
}
