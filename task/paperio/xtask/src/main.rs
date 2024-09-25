use std::{
    fs, process,
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};
use xtask_util::get_cwd_task_path;

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

    /// Run gui and play against bots.
    Play,

    /// Run your strategy and gui to look how it plays.
    Watch,

    /// Wait for your strategy to connect on port 8004 and then run the game.
    Debug,

    /// Run you strategy three times against bots (no gui).
    Challenge,
}

#[derive(Clone, Copy)]
enum GuiMode {
    None,
    Spectator,
    Player,
}

enum Outcome {
    Won,
    Lost,
}

struct Recipe {
    gui_mode: GuiMode,
    run_strategy: bool,
}

impl Recipe {
    fn run(&self) -> Result<()> {
        Self::build_binaries()?;

        let server_handle = match self.gui_mode {
            GuiMode::Spectator => Self::launch_server(true),
            _ => Self::launch_server(false),
        };

        let bot_handles = Self::launch_bots()?;

        let gui_handle = match self.gui_mode {
            GuiMode::None => None,
            GuiMode::Spectator => Some(Self::launch_gui(true)),
            GuiMode::Player => Some(Self::launch_gui(false)),
        };

        let strategy_handle = if self.run_strategy {
            Some(Self::launch_strategy())
        } else {
            None
        };

        for handle in bot_handles {
            let _ = handle.join();
        }

        for mb_handle in [gui_handle, strategy_handle] {
            if let Some(handle) = mb_handle {
                let _ = handle.join();
            }
        }

        match server_handle.join() {
            Ok(Ok(Outcome::Won)) => eprintln!("Congratulations, you won!"),
            Ok(Ok(Outcome::Lost)) => {
                eprintln!("You lost :(");
                bail!("strategy lost");
            }
            Ok(Err(err)) => {
                bail!("server error: {err}")
            }
            Err(_) => {
                bail!("server panicked")
            }
        }

        Ok(())
    }

    fn build_binaries() -> Result<()> {
        let sh = Shell::new()?;
        for package in [
            "paperio-wasm-launcher",
            "paperio-strategy",
            "paperio-gui",
            "paperio-server",
        ] {
            cmd!(sh, "cargo build --package {package} --release").run()?;
        }
        Ok(())
    }

    fn launch_bots() -> Result<Vec<JoinHandle<Result<()>>>> {
        let mut handles = Vec::<JoinHandle<Result<()>>>::with_capacity(3);

        let bot_names = ["fool", "coward", "aggressive"];

        for bot_name in bot_names {
            let bot_path = get_cwd_task_path()?
                .join("bots")
                .join(format!("{bot_name}.wasm"));

            let handle = thread::spawn(move || -> Result<()> {
                let mut cmd = process::Command::new("cargo");
                cmd.args([
                    "run",
                    "--package",
                    "paperio-wasm-launcher",
                    "--release",
                    "--",
                ])
                .arg(bot_path);

                Self::run_with_log(cmd, &format!("bot_{bot_name}"))?;

                Ok(())
            });

            handles.push(handle);
        }
        Ok(handles)
    }

    fn launch_strategy() -> JoinHandle<Result<()>> {
        thread::spawn(|| -> Result<()> {
            let mut cmd = process::Command::new("cargo");
            cmd.args([
                "run",
                "--package",
                "paperio-strategy",
                "--release",
                "--",
                "8004",
            ]);

            Self::run_with_log(cmd, "strategy")?;

            Ok(())
        })
    }

    fn launch_gui(is_spectator: bool) -> JoinHandle<Result<()>> {
        thread::spawn(move || -> Result<()> {
            let (port, spectator_arg) = if is_spectator {
                ("8001", &["--spectator"] as &[_])
            } else {
                ("8004", &[] as &[_])
            };

            let mut cmd = process::Command::new("cargo");
            cmd.args([
                "run",
                "--package",
                "paperio-gui",
                "--release",
                "--",
                "-p",
                port,
            ])
            .args(spectator_arg);

            Self::run_with_log(cmd, "gui")?;

            Ok(())
        })
    }

    fn launch_server(with_spectator: bool) -> JoinHandle<Result<Outcome>> {
        let handle = thread::spawn(move || -> Result<Outcome> {
            let mut cmd = process::Command::new("cargo");
            cmd.args([
                "run",
                "--package",
                "paperio-server",
                "--release",
                "--",
                "--p4",
                "8004",
            ]);

            if with_spectator {
                cmd.args(["--spectator-count", "1"]);
            }

            let stdout = Self::run_with_log(cmd, "server")?;

            if String::from_utf8_lossy(&stdout).contains("Winner is Player #4") {
                Ok(Outcome::Won)
            } else {
                Ok(Outcome::Lost)
            }
        });

        thread::sleep(Duration::from_millis(500));
        handle
    }

    fn run_with_log(mut cmd: process::Command, log_name: &str) -> Result<Vec<u8>> {
        let dir_path = get_cwd_task_path()?.join("log");
        if !dir_path.exists() {
            fs::create_dir(&dir_path).context("failed to create log dir")?;
        }

        let file_path = dir_path.join(format!("{log_name}.log"));
        let log_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)
            .with_context(|| format!("failed to create {file_path:?}"))?;

        eprintln!("Running: {cmd:?}");
        let output = cmd
            .stderr(log_file)
            .output()
            .with_context(|| format!("failed to run {cmd:?}"))?;

        eprintln!("Terminated: {cmd:?}");
        Ok(output.stdout)
    }
}

fn play() -> Result<()> {
    Recipe {
        gui_mode: GuiMode::Player,
        run_strategy: false,
    }
    .run()
}

fn watch() -> Result<()> {
    Recipe {
        gui_mode: GuiMode::Spectator,
        run_strategy: true,
    }
    .run()
}

fn debug() -> Result<()> {
    Recipe {
        gui_mode: GuiMode::Spectator,
        run_strategy: false,
    }
    .run()
}

fn challenge() -> Result<()> {
    for i in 1..=3 {
        eprintln!("Running test #{i}...");

        Recipe {
            gui_mode: GuiMode::None,
            run_strategy: true,
        }
        .run()?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Base(cmd) => xtask_base::run_command(cmd),
        Command::Play => play(),
        Command::Watch => watch(),
        Command::Debug => debug(),
        Command::Challenge => challenge(),
    }
}
