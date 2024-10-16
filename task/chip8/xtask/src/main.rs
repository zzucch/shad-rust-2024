use std::{path::Path, process};

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand, ValueEnum};

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

    /// Run your interpreter in terminal.
    Run {
        /// Path to image.
        image_path: String,
    },

    /// Run one specific test in terminal.
    #[command(arg_required_else_help = true)]
    RunTest {
        #[arg(value_enum)]
        test_image: TestImage,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum TestImage {
    Chip8Logo,
    IBMLogo,
    Corax,
    Flags,
    Quirks,
    Keypad,
}

fn run(image_path: impl AsRef<Path>) -> Result<()> {
    let status = process::Command::new("cargo")
        .args(["run", "--package", "chip8-console-runner", "--"])
        .arg(image_path.as_ref())
        .status()?;
    ensure!(status.success(), "command exited with status {}", status);
    Ok(())
}

fn run_test(test_image: TestImage) -> Result<()> {
    let task_path = xtask_util::get_cwd_task_path()?
        .join("images/tests")
        .join(match test_image {
            TestImage::Chip8Logo => "1-chip8-logo.ch8",
            TestImage::IBMLogo => "2-ibm-logo.ch8",
            TestImage::Corax => "3-corax+.ch8",
            TestImage::Flags => "4-flags.ch8",
            TestImage::Quirks => "5-quirks.ch8",
            TestImage::Keypad => "6-keypad.ch8",
        });
    run(task_path)
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Base(cmd) => xtask_base::run_command(cmd),
        Command::Run { image_path } => run(image_path),
        Command::RunTest { test_image } => run_test(test_image),
    }
}
