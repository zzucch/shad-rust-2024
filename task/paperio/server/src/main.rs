mod game;
mod game_field;
mod player_endpoint;
mod player_vec;
mod server;

use clap::Parser;
use server::Server;
use std::io;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[arg(short, long, default_value_t = String::from("localhost"))]
    address: String,

    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    #[arg(short = 'n', long, default_value_t = 4)]
    players_amount: usize,

    #[arg(short, long, default_value_t = 300)]
    ticks_amount: usize,

    #[arg(long, default_value_t = 8001)]
    spectator_port: u16,

    #[arg(short, long, default_value_t = 0)]
    spectators_amount: usize,

    #[arg(short, long, default_value_t = 2)]
    log_level: usize,
}

fn main() -> io::Result<()> {
    let args = Arguments::parse();

    stderrlog::new()
        .verbosity(args.log_level)
        .module(module_path!())
        .init()
        .unwrap();

    let address = format!("{}:{}", args.address, args.port);
    let spectators_address = format!("{}:{}", args.address, args.spectator_port);
    let mut server = Server::new(&address, &spectators_address)?;
    server.start(
        args.ticks_amount,
        args.players_amount,
        args.spectators_amount,
    )?;

    Ok(())
}
