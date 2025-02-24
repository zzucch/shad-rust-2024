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

    #[arg(long, action)]
    /// Don't capture logs to log/.
    no_logs: bool,
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
    capture_logs: bool,
}

impl Recipe {
    fn run(&self) -> Result<()> {
        Self::build_binaries()?;

        let server_handle = match self.gui_mode {
            GuiMode::Spectator => Self::launch_server(true, self.capture_logs),
            _ => Self::launch_server(false, self.capture_logs),
        };

        let bot_handles = Self::launch_bots(self.capture_logs)?;

        let gui_handle = match self.gui_mode {
            GuiMode::None => None,
            GuiMode::Spectator => Some(Self::launch_gui(true, self.capture_logs)),
            GuiMode::Player => Some(Self::launch_gui(false, self.capture_logs)),
        };

        let strategy_handle = if self.run_strategy {
            Some(Self::launch_strategy(self.capture_logs))
        } else {
            None
        };

        for handle in bot_handles {
            let _ = handle.join();
        }

        for handle in [gui_handle, strategy_handle].into_iter().flatten() {
            let _ = handle.join();
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

    fn launch_bots(capture_logs: bool) -> Result<Vec<JoinHandle<Result<()>>>> {
        let mut handles = Vec::<JoinHandle<Result<()>>>::with_capacity(3);

        let bot_names = ["coward", "coward", "coward"];

        for (bot_id, bot_name) in bot_names.iter().enumerate() {
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

                let log_name = if capture_logs {
                    Some(format!("bot_{bot_id}"))
                } else {
                    None
                };
                Self::run_cmd(cmd, log_name)?;

                Ok(())
            });

            handles.push(handle);
        }
        Ok(handles)
    }

    fn launch_strategy(capture_logs: bool) -> JoinHandle<Result<()>> {
        thread::spawn(move || -> Result<()> {
            let mut cmd = process::Command::new("cargo");
            cmd.args([
                "run",
                "--package",
                "paperio-strategy",
                "--release",
                "--",
                "8004",
            ]);

            let log_name = if capture_logs { Some("strategy") } else { None };
            Self::run_cmd(cmd, log_name)?;

            Ok(())
        })
    }

    fn launch_gui(is_spectator: bool, capture_logs: bool) -> JoinHandle<Result<()>> {
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

            let log_name = if capture_logs { Some("gui") } else { None };
            Self::run_cmd(cmd, log_name)?;

            Ok(())
        })
    }

    fn launch_server(with_spectator: bool, capture_logs: bool) -> JoinHandle<Result<Outcome>> {
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

            let log_name = if capture_logs { Some("server") } else { None };
            let stdout = Self::run_cmd(cmd, log_name)?;

            if String::from_utf8_lossy(&stdout).contains("Winner is Player #4") {
                Ok(Outcome::Won)
            } else {
                Ok(Outcome::Lost)
            }
        });

        thread::sleep(Duration::from_millis(500));
        handle
    }

    fn run_cmd(mut cmd: process::Command, log_name: Option<impl AsRef<str>>) -> Result<Vec<u8>> {
        if let Some(log_name) = log_name {
            let dir_path = get_cwd_task_path()?.join("log");
            if !dir_path.exists() {
                fs::create_dir(&dir_path).context("failed to create log dir")?;
            }

            let file_path = dir_path.join(format!("{}.log", log_name.as_ref()));
            let log_file = fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&file_path)
                .with_context(|| format!("failed to create {file_path:?}"))?;

            cmd.stderr(log_file);
        }

        eprintln!(
            "$ {} {}",
            cmd.get_program().to_string_lossy(),
            cmd.get_args()
                .into_iter()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );

        let output = cmd
            .output()
            .with_context(|| format!("failed to run {cmd:?}"))?;

        Ok(output.stdout)
    }
}

fn play(no_logs: bool) -> Result<()> {
    Recipe {
        gui_mode: GuiMode::Player,
        run_strategy: false,
        capture_logs: !no_logs,
    }
    .run()
}

fn watch(no_logs: bool) -> Result<()> {
    Recipe {
        gui_mode: GuiMode::Spectator,
        run_strategy: true,
        capture_logs: !no_logs,
    }
    .run()
}

fn debug(no_logs: bool) -> Result<()> {
    Recipe {
        gui_mode: GuiMode::Spectator,
        run_strategy: false,
        capture_logs: !no_logs,
    }
    .run()
}

fn challenge(no_logs: bool) -> Result<()> {
    for i in 1..=3 {
        eprintln!("Running test #{i}...");

        Recipe {
            gui_mode: GuiMode::None,
            run_strategy: true,
            capture_logs: !no_logs,
        }
        .run()?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Base(cmd) => xtask_base::run_command(cmd),
        Command::Play => play(args.no_logs),
        Command::Watch => watch(args.no_logs),
        Command::Debug => debug(args.no_logs),
        Command::Challenge => challenge(args.no_logs),
    }
}
