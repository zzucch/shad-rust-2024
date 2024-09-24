use std::{
    io::{self, BufReader, BufWriter},
    net::{TcpListener, ToSocketAddrs},
};

use log::*;

use crate::{
    game::{Game, PlayerId},
    player_endpoint::PlayerEndpoint,
    player_vec::PlayerIndexedVector,
};

pub struct Server {
    tcp_listener: TcpListener,
    spectators_tcp_listener: TcpListener,
}

impl Server {
    pub fn new(addr: impl ToSocketAddrs, spec_addr: impl ToSocketAddrs) -> io::Result<Server> {
        let tcp_listener = TcpListener::bind(addr)?;
        let spectators_tcp_listener = TcpListener::bind(spec_addr)?;
        Ok(Self {
            tcp_listener,
            spectators_tcp_listener,
        })
    }

    fn collect_players(listener: &mut TcpListener, amount: usize) -> Vec<PlayerEndpoint> {
        listener
            .incoming()
            .filter_map(|stream| {
                let stream = stream.ok()?;
                let player_addr = stream.peer_addr().ok()?;
                let reader = BufReader::new(stream.try_clone().ok()?);
                let writer = BufWriter::new(stream);
                let player = PlayerEndpoint::new(reader, writer);
                info!("Player {player_addr} connected!");
                Some(player)
            })
            .take(amount)
            .collect::<Vec<PlayerEndpoint>>()
    }

    pub fn start(
        &mut self,
        ticks_amount: usize,
        players_amount: usize,
        spectators_amount: usize,
    ) -> io::Result<PlayerId> {
        info!("Starting server at {}", self.tcp_listener.local_addr()?);
        let players = Self::collect_players(&mut self.tcp_listener, players_amount);
        let mut players = PlayerIndexedVector::from(players);

        let mut spectators =
            Self::collect_players(&mut self.spectators_tcp_listener, spectators_amount);

        let mut game = Game::new(players.len());
        let params = game.get_game_params();
        for (i, p) in players.iter_mut() {
            if let Err(err) = p.send_start_game(params) {
                error!("Sending start-game-message to Player #{i}: {err}");
            }
        }
        for s in &mut spectators {
            let _ = s.send_start_game(params);
        }

        for tick in 0..ticks_amount {
            debug!("tick #{tick}");
            for (i, p) in players.iter_mut() {
                if let Err(err) = p.send_tick(game.get_player_world(i)) {
                    error!("Sending tick to Player #{i}: {err}");
                }
            }
            for s in &mut spectators {
                let _ = s.send_tick(game.get_spectator_world());
            }

            for (i, p) in players.iter_mut() {
                match p.get_tick() {
                    Ok(dir) => {
                        game.try_change_direction(i, dir.command);
                    }
                    Err(err) => {
                        error!("Getting command from Player #{i}: {err}")
                    }
                }
            }
            for s in &mut spectators {
                let _ = s.get_tick();
            }

            game.tick();
        }
        for (i, p) in players.iter_mut() {
            if let Err(err) = p.send_end_game() {
                error!("Sending end-game-message to Player #{i}: {err}");
            };
        }
        for s in &mut spectators {
            let _ = s.send_end_game();
        }
        let winner = game.leader_id();
        info!("Winner is Player #{winner}!");
        Ok(winner)
    }
}
