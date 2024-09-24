use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{anyhow, Result};
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

    /// Run gui and play against bots.
    Play,

    /// Run your strategy and gui to look how it plays.
    Watch,

    /// Run you strategy three times against bots (no gui).
    Challenge,
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

    let bots = ["bots/fool.wasm", "bots/coward.wasm", "bots/aggressive.wasm"];

    for bot in bots {
        handles.push(thread::spawn(move || -> Result<()> {
            let bot_sh = Shell::new()?;
            cmd!(
                bot_sh,
                "cargo run --package paperio-wasm-launcher --release --"
            )
            .arg(bot)
            .ignore_stdout()
            .ignore_stderr()
            .ignore_status()
            .run()?;
            Ok(())
        }));
        thread::sleep(Duration::from_millis(100));
    }
    Ok(handles)
}

fn launch_strategy() -> JoinHandle<Result<()>> {
    thread::spawn(|| -> Result<()> {
        let strategy_sh = Shell::new()?;
        cmd!(
            strategy_sh,
            "cargo run --package paperio-strategy --release -- 8000"
        )
        .run()?;
        Ok(())
    })
}

fn launch_gui(is_spectator: bool) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        let gui_sh = Shell::new()?;
        cmd!(gui_sh, "cargo run --package paperio-gui --release --")
            .arg("-p")
            .arg(if is_spectator { "8001" } else { "8000" })
            .run()?;
        Ok(())
    })
}

fn launch_server(with_spectator: bool) -> JoinHandle<Result<bool>> {
    let handle = thread::spawn(move || -> Result<bool> {
        let server_sh = Shell::new()?;
        let mut cmd = cmd!(server_sh, "cargo run --release --package paperio-server --");
        if with_spectator {
            cmd = cmd.arg("-s").arg("1");
        }
        let output = cmd.output()?;
        let output = String::from_utf8(output.stderr)?;
        let is_winner = output.lines().any(|l| l.contains("Winner is Player #4"));
        Ok(is_winner)
    });
    thread::sleep(Duration::from_millis(100));
    handle
}

fn play() -> Result<()> {
    build_binaries()?;

    let server_handle = launch_server(false);
    let bot_handles = launch_bots()?;
    let gui_handle = launch_gui(false);

    for handle in bot_handles {
        let _ = handle.join();
    }

    let _ = gui_handle.join();

    match server_handle.join() {
        Ok(Ok(true)) => eprintln!("Congratulations, you won!"),
        Ok(Ok(false)) => eprintln!("You lost :("),
        Ok(Err(err)) => eprintln!("Server error: {err}"),
        Err(_) => eprintln!("Could not join server thread"),
    }

    Ok(())
}

fn watch() -> Result<()> {
    build_binaries()?;

    let server_handle = launch_server(true);
    let bot_handles = launch_bots()?;
    thread::sleep(Duration::from_millis(500));
    let strategy_handle = launch_strategy();
    let gui_handle = launch_gui(true);

    for handle in bot_handles {
        let _ = handle.join();
    }

    match strategy_handle.join() {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => eprintln!("Strategy error: {err}"),
        Err(_) => eprintln!("Could not join strategy thread"),
    }

    let _ = gui_handle.join();

    match server_handle.join() {
        Ok(Ok(true)) => eprintln!("Your strategy won!"),
        Ok(Ok(false)) => eprintln!("Your strategy lost :("),
        Ok(Err(err)) => eprintln!("Server error: {err}"),
        Err(_) => eprintln!("Could not join server thread"),
    }

    Ok(())
}

fn test_once() -> Result<()> {
    let server_handle = launch_server(false);
    let bot_handles = launch_bots()?;
    thread::sleep(Duration::from_millis(500));
    let strategy_handle = launch_strategy();

    for handle in bot_handles {
        let _ = handle.join();
    }

    match strategy_handle.join() {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => eprintln!("Strategy error: {err}"),
        Err(_) => eprintln!("Could not join strategy thread"),
    }

    match server_handle.join() {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err(anyhow!("Your strategy lost :(")),
        Ok(Err(err)) => Err(anyhow!("Server error: {err}")),
        Err(_) => Err(anyhow!("Could not join server thread")),
    }
}

fn challenge() -> Result<()> {
    build_binaries()?;

    for i in 1..=3 {
        eprintln!("Running test #{i}...");
        test_once()?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Base(cmd) => xtask_base::run_command(cmd),
        Command::Play => play(),
        Command::Watch => watch(),
        Command::Challenge => challenge(),
    }
}
