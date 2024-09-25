use std::{collections::HashMap, num::NonZero};

use paperio_proto::{self, Cell, Direction, GameParams, World};

use crate::{game_field::GameField, player_vec::PlayerIndexedVector};

const INIT_POS: [Cell; 4] = [Cell(9, 21), Cell(21, 21), Cell(21, 9), Cell(9, 9)];
const X_CELLS_COUNT: u32 = 31;
const Y_CELLS_COUNT: u32 = 31;

pub type PlayerId = NonZero<usize>;

struct Player {
    score: u32,
    position: Cell,
    direction: Direction,
}

impl Player {
    fn new(position: Cell) -> Self {
        Player {
            score: 0,
            position,
            direction: Direction::Left,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum PlayerStatus {
    #[default]
    Alive,
    Losing,
    Lost,
}

impl PlayerStatus {
    fn has_lost(&self) -> bool {
        matches!(self, Self::Losing | Self::Lost)
    }
}

pub struct Game {
    tick: u32,
    players: PlayerIndexedVector<Player>,
    player_status: PlayerIndexedVector<PlayerStatus>,
    params: GameParams,
    field: GameField,
}

impl Game {
    pub fn new(players_amount: usize) -> Self {
        let params = GameParams {
            x_cells_count: X_CELLS_COUNT,
            y_cells_count: Y_CELLS_COUNT,
        };
        let mut field = GameField::new(
            params.x_cells_count as usize,
            params.y_cells_count as usize,
            players_amount,
        );
        let players: PlayerIndexedVector<Player> = INIT_POS
            .iter()
            .map(|&pos| Player::new(pos))
            .take(players_amount)
            .collect::<Vec<_>>()
            .into();

        for (player_id, player) in players.iter() {
            field.init_player(player_id, player.position);
        }
        let player_status = PlayerIndexedVector::new(players_amount);

        Game {
            tick: 1,
            players,
            player_status,
            params,
            field,
        }
    }

    pub fn has_lost(&self, i: PlayerId) -> bool {
        self.player_status[i].has_lost()
    }

    pub fn get_game_params(&self) -> GameParams {
        self.params
    }

    pub fn try_change_direction(&mut self, player_id: PlayerId, new_direction: Direction) -> bool {
        let direction = &mut self.players[player_id].direction;
        if new_direction == direction.opposite() {
            return false;
        }
        *direction = new_direction;
        true
    }

    pub fn tick(&mut self) {
        let next_position = self
            .players
            .map(|player| player.position + player.direction);

        let mut cell_to_contenders = HashMap::<Cell, Vec<PlayerId>>::new();
        for (player_id, &next_position) in next_position.iter() {
            if self.player_status[player_id].has_lost() {
                continue;
            }

            if !next_position.in_bounds() {
                self.player_status[player_id] = PlayerStatus::Losing;
            } else {
                cell_to_contenders
                    .entry(next_position)
                    .or_default()
                    .push(player_id);
            }
        }

        for (&pos, players) in cell_to_contenders.iter() {
            if players.len() > 1 {
                for &player_id in players {
                    if !self.field[pos].is_captured_by(player_id) {
                        self.player_status[player_id] = PlayerStatus::Losing;
                    }
                }
            }
        }

        let player_positions = self.players.map(|p| p.position);
        for (player_id, player) in self.players.iter_mut() {
            if self.player_status[player_id].has_lost() {
                continue;
            }

            let cell_state = &self.field[next_position[player_id]];
            if cell_state.is_traced_by(player_id) {
                // self-cross
                self.player_status[player_id] = PlayerStatus::Losing;
                continue;
            } else if cell_state.is_captured_by(player_id) {
                let (enemy_cells_captured, free_cells_captured, enemies_captured) =
                    self.field.capture_all(player_id, &player_positions);

                player.score += enemy_cells_captured * 5 + free_cells_captured;

                for &enemy_id in &enemies_captured {
                    self.player_status[enemy_id] = PlayerStatus::Losing;
                }
            }
        }

        for (player_id, _) in self.players.iter_mut() {
            if self.player_status[player_id].has_lost() {
                continue;
            }

            let cell_state = self.field[next_position[player_id]];
            if let Some(traced_player_id) = cell_state.is_traced() {
                self.player_status[traced_player_id] = PlayerStatus::Losing;
            }
        }

        for (player_id, _) in self.players.iter_mut() {
            if self.player_status[player_id].has_lost() {
                continue;
            }

            let cell_state = self.field[next_position[player_id]];
            if !cell_state.is_captured_by(player_id) {
                self.field.set_trace(next_position[player_id], player_id)
            }
        }

        for (player_id, status) in self.player_status.iter_mut() {
            match *status {
                PlayerStatus::Alive => {
                    self.players[player_id].position = next_position[player_id];
                }
                PlayerStatus::Losing => {
                    self.field.remove_player(player_id);
                    *status = PlayerStatus::Lost;
                }
                PlayerStatus::Lost => {}
            }
        }

        self.tick += 1;
    }

    pub fn get_player_world(&self, i: PlayerId) -> World {
        let players = self
            .players
            .iter()
            .map(|(id, player)| {
                let str_id = if id == i {
                    "i".to_string()
                } else {
                    id.get().to_string()
                };

                let (territory, lines) = self.field.get_for_player(id);
                let proto_player = paperio_proto::Player {
                    score: player.score,
                    territory: territory.iter().copied().collect(),
                    position: player.position,
                    lines: lines.iter().copied().collect(),
                    direction: Some(player.direction),
                    has_lost: self.has_lost(id),
                };

                (str_id, proto_player)
            })
            .collect();
        World {
            players,
            tick_num: self.tick,
        }
    }

    pub fn get_spectator_world(&self) -> World {
        self.get_player_world(NonZero::new(usize::MAX).unwrap())
    }

    pub fn leader_id(&self) -> Option<PlayerId> {
        let player_id = self
            .players
            .iter()
            .max_by_key(|(_, player)| player.score)
            .unwrap()
            .0;
        let leader_score = self.players[player_id].score;
        if self
            .players
            .iter()
            .filter(|p| p.1.score == leader_score)
            .count()
            > 1
        {
            None
        } else {
            Some(player_id)
        }
    }
}
