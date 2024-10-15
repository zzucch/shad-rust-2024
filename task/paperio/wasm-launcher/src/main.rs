use anyhow::{Context, Result};
use clap::Parser;
use paperio_wasm_launcher::WasmStrategyRunner;

use std::net::TcpStream;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    path: String,
    #[arg(short, long, default_value_t = String::from("localhost"))]
    address: String,
    #[arg(short, long, default_value_t = 8000)]
    port: u16,
}

pub fn main() -> Result<()> {
    let args = Arguments::parse();

    let address = format!("{}:{}", args.address, args.port);
    let stdin = TcpStream::connect(&address).with_context(|| format!("failed to {address}"))?;
    let stdout = stdin.try_clone().context("failed to clone tcp stream")?;

    let status = WasmStrategyRunner::new(&args.path, stdin, stdout)
        .run()
        .context("failed to run strategy")?;

    status.result.context("strategy failed")
}
