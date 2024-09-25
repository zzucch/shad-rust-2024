use std::io;

use log::*;
use paperio_proto::{Command, Message};

use crate::{
    endpoint::Endpoint,
    game::{Game, PlayerId},
    player_vec::PlayerIndexedVector,
};

pub struct Server {
    player_endpoints: PlayerIndexedVector<Box<dyn Endpoint>>,
    spectator_endpoints: Vec<Box<dyn Endpoint>>,
}

impl Server {
    pub fn new(
        player_endpoints: PlayerIndexedVector<impl Endpoint + 'static>,
        spectator_endpoints: impl IntoIterator<Item = impl Endpoint + 'static>,
    ) -> Self {
        Self {
            player_endpoints: player_endpoints.mapped(|e| Box::new(e) as Box<dyn Endpoint>),
            spectator_endpoints: spectator_endpoints
                .into_iter()
                .map(|e| Box::new(e) as Box<dyn Endpoint>)
                .collect(),
        }
    }

    pub fn run(&mut self, ticks_amount: usize) -> io::Result<Option<PlayerId>> {
        let mut game = Game::new(self.player_endpoints.len());
        let params = game.get_game_params();

        self.send_to_all(&Message::StartGame(params));

        for tick in 0..ticks_amount {
            debug!("tick #{tick}");

            for player_id in self.player_endpoints.iter_player_ids() {
                if !game.has_lost(player_id) {
                    let world = game.get_player_world(player_id);
                    self.send_to_player(player_id, &Message::Tick(world));
                }
            }

            let spectator_world = game.get_spectator_world();
            self.send_to_spectators(&Message::Tick(spectator_world));

            for player_id in self.player_endpoints.iter_player_ids() {
                if !game.has_lost(player_id) {
                    let mb_command = self.try_get_player_command(player_id);
                    if let Some(Command::ChangeDirection(dir)) = mb_command {
                        game.try_change_direction(player_id, dir);
                    }
                }
            }

            self.sync_with_spectators();

            game.tick();
        }

        self.send_to_all(&Message::EndGame {});

        let mb_leader_id = game.leader_id();
        match mb_leader_id {
            Some(player_id) => info!("Winner is Player #{player_id}!"),
            None => info!("There is no winner (tie)"),
        }

        Ok(mb_leader_id)
    }

    fn send_to_spectators(&mut self, message: &Message) {
        for endpoint in self.spectator_endpoints.iter_mut() {
            if let Err(err) = endpoint.send_message(message) {
                error!("failed to send message to spectator: {err}");
            }
        }
    }

    fn send_to_player(&mut self, player_id: PlayerId, message: &Message) {
        let endpoint = &mut self.player_endpoints[player_id];
        if let Err(err) = endpoint.send_message(message) {
            error!("failed to send message to Player #{player_id}: {err}");
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
        let endpoint = &mut self.player_endpoints[player_id];
        match endpoint.get_command() {
            Ok(cmd) => Some(cmd),
            Err(err) => {
                error!("failed to get command from Player #{player_id}: {err}");
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
