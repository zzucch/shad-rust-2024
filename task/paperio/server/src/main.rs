mod endpoint;
mod game;
mod game_field;
mod player_vec;
mod server;

use anyhow::{ensure, Context, Result};
use clap::Parser;
use endpoint::{Endpoint, JsonEndpoint};
use game::PlayerId;
use log::info;
use player_vec::PlayerIndexedVector;
use server::Server;

use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    iter,
    net::{SocketAddr, TcpListener},
    thread,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
    address: String,

    #[arg(short = 'p', long, default_value_t = 8000)]
    default_player_port: u16,

    #[arg(long = "p1")]
    player_one_port: Option<u16>,

    #[arg(long = "p2")]
    player_two_port: Option<u16>,

    #[arg(long = "p3")]
    player_three_port: Option<u16>,

    #[arg(long = "p4")]
    player_four_port: Option<u16>,

    #[arg(short = 'n', long, default_value_t = 4)]
    player_count: usize,

    #[arg(short, long, default_value_t = 300)]
    tick_count: usize,

    #[arg(long, default_value_t = 8001)]
    spectator_port: u16,

    #[arg(short, long, default_value_t = 0)]
    spectator_count: usize,

    #[arg(short, long, default_value_t = 2)]
    log_level: usize,
}

#[derive(Clone, Copy)]
enum EndpointTag {
    Player(PlayerId),
    Spectator,
}

fn get_port_to_endpoint_tags(args: &Arguments) -> HashMap<u16, Vec<EndpointTag>> {
    let player_ports = [
        args.player_one_port,
        args.player_two_port,
        args.player_three_port,
        args.player_four_port,
    ];

    let mut port_to_endpoint_tags = HashMap::<u16, Vec<EndpointTag>>::new();

    for i in 0..args.player_count {
        let tag = EndpointTag::Player(PlayerId::new(i + 1).unwrap());

        let port = player_ports
            .get(i)
            .copied()
            .flatten()
            .unwrap_or(args.default_player_port);

        port_to_endpoint_tags
            .entry(port)
            .and_modify(|tags| tags.push(tag))
            .or_insert(vec![tag]);
    }

    port_to_endpoint_tags
        .entry(args.spectator_port)
        .and_modify(|tags| {
            tags.extend(iter::repeat(EndpointTag::Spectator).take(args.spectator_count))
        })
        .or_insert(vec![EndpointTag::Spectator; args.spectator_count]);

    port_to_endpoint_tags
}

fn spawn_listener(
    socket_address: SocketAddr,
    tags: Vec<EndpointTag>,
) -> thread::JoinHandle<Result<Vec<(EndpointTag, impl Endpoint)>>> {
    thread::spawn(move || {
        if tags.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "waiting for {} connection(s) on {socket_address} ...",
            tags.len()
        );

        tags.into_iter()
            .zip(TcpListener::bind(socket_address)?.incoming())
            .map(|(tag, mb_stream)| {
                let stream = mb_stream?;
                let peer_addr = stream.peer_addr()?;
                info!("incomming connection: {peer_addr} -> {socket_address}");

                let reader = BufReader::new(stream.try_clone().context("failed to clone fd")?);
                let writer = BufWriter::new(stream);
                let endpoint = JsonEndpoint::new(reader, writer);

                Ok((tag, endpoint))
            })
            .collect()
    })
}

fn get_endpoints(
    args: &Arguments,
) -> Result<(PlayerIndexedVector<impl Endpoint>, Vec<impl Endpoint>)> {
    let port_to_endpoint_tags = get_port_to_endpoint_tags(args);

    let mut handles = vec![];
    for (port, endpoint_tags) in port_to_endpoint_tags {
        let socket_addr = format!("{}:{}", args.address, port)
            .parse()
            .with_context(|| format!("invalid socket address: {}:{}", args.address, port))?;
        let handle = spawn_listener(socket_addr, endpoint_tags);
        handles.push(handle);
    }

    let mut players = PlayerIndexedVector::new(args.player_count);
    let mut spectators = vec![];
    for handle in handles {
        for (tag, endpoint) in handle.join().unwrap()? {
            match tag {
                EndpointTag::Player(player_id) => players[player_id] = Some(endpoint),
                EndpointTag::Spectator => spectators.push(endpoint),
            }
        }
    }

    let players = players.mapped(|e| e.unwrap());

    Ok((players, spectators))
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    ensure!(
        (1..=4).contains(&args.player_count),
        "player count should be from 1 to 4"
    );

    stderrlog::new()
        .verbosity(args.log_level)
        .module(module_path!())
        .init()
        .unwrap();

    let (player_endpoints, spectator_endpoints) = get_endpoints(&args)?;
    Server::new(player_endpoints, spectator_endpoints).run(args.tick_count)?;

    Ok(())
}
