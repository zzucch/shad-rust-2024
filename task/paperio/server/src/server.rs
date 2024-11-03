use std::io;

use log::*;
use paperio_proto::{Command, Message};

use crate::{
    endpoint::Endpoint,
    game::{Game, PlayerId},
    player_vec::PlayerIndexedVector,
};

pub struct PlayerResult {
    pub score: u32,
    pub io_error: Option<io::Error>,
}

pub struct Server<'a> {
    player_endpoints: PlayerIndexedVector<Box<dyn Endpoint + 'a>>,
    spectator_endpoints: Vec<Box<dyn Endpoint + 'a>>,
    player_io_errors: PlayerIndexedVector<Option<io::Error>>,
}

impl<'a> Server<'a> {
    pub fn new(
        player_endpoints: PlayerIndexedVector<impl Endpoint + 'a>,
        spectator_endpoints: impl IntoIterator<Item = impl Endpoint + 'a>,
    ) -> Self {
        let player_count = player_endpoints.len();
        Self {
            player_endpoints: player_endpoints.mapped(|e| Box::new(e) as Box<dyn Endpoint>),
            spectator_endpoints: spectator_endpoints
                .into_iter()
                .map(|e| Box::new(e) as Box<dyn Endpoint>)
                .collect(),
            player_io_errors: PlayerIndexedVector::new(player_count),
        }
    }

    pub fn run(mut self, ticks_amount: usize) -> PlayerIndexedVector<PlayerResult> {
        let mut game = Game::new(self.player_endpoints.len());
        let params = game.get_game_params();

        self.send_to_all(&Message::StartGame(params));

        for tick in 0..ticks_amount {
            debug!("tick #{tick}");

            for player_id in self.player_endpoints.iter_player_ids() {
                let world = game.get_player_world(player_id);
                self.send_to_player(player_id, &Message::Tick(world));
            }

            let spectator_world = game.get_spectator_world();
            self.send_to_spectators(&Message::Tick(spectator_world));

            for player_id in self.player_endpoints.iter_player_ids() {
                let mb_command = self.try_get_player_command(player_id);
                if let Some(Command::ChangeDirection(dir)) = mb_command {
                    game.try_change_direction(player_id, dir);
                }
            }

            self.sync_with_spectators();

            game.tick();
        }

        self.send_to_all(&Message::EndGame {});

        let mb_leader_id = game.leader_id();
        match mb_leader_id {
            Some(player_id) => println!("Winner is Player #{player_id}!"),
            None => println!("There is no winner (tie)"),
        }

        game.get_player_scores()
            .into_iter()
            .zip(self.player_io_errors)
            .map(|(score, io_error)| PlayerResult { score, io_error })
            .collect::<Vec<_>>()
            .into()
    }

    fn send_to_spectators(&mut self, message: &Message) {
        for endpoint in self.spectator_endpoints.iter_mut() {
            if let Err(err) = endpoint.send_message(message) {
                error!("failed to send message to spectator: {err}");
            }
        }
    }

    fn send_to_player(&mut self, player_id: PlayerId, message: &Message) {
        if self.player_io_errors[player_id].is_some() {
            return;
        }
        let endpoint = &mut self.player_endpoints[player_id];
        if let Err(err) = endpoint.send_message(message) {
            error!("failed to send message to Player #{player_id}: {err}");
            self.player_io_errors[player_id] = Some(err);
        }
    }

    fn send_to_players(&mut self, message: &Message) {
        for player_id in self.player_endpoints.iter_player_ids() {
            self.send_to_player(player_id, message);
        }
    }

    fn send_to_all(&mut self, message: &Message) {
        self.send_to_players(message);
        self.send_to_spectators(message);
    }

    fn try_get_player_command(&mut self, player_id: PlayerId) -> Option<Command> {
        if self.player_io_errors[player_id].is_some() {
            return None;
        }
        let endpoint = &mut self.player_endpoints[player_id];
        match endpoint.get_command() {
            Ok(cmd) => Some(cmd),
            Err(err) => {
                error!("failed to get command from Player #{player_id}: {err}");
                self.player_io_errors[player_id] = Some(err);
                None
            }
        }
    }

    fn sync_with_spectators(&mut self) {
        for endpoint in self.spectator_endpoints.iter_mut() {
            if let Err(err) = endpoint.get_command() {
                error!("failed to sync with spectator: {err}");
            }
        }
    }
}
