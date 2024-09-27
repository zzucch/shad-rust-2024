use std::{cmp::Ordering, collections::HashMap, num::NonZero};

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

pub struct Game {
    tick: u32,
    players: PlayerIndexedVector<Player>,
    has_lost: PlayerIndexedVector<bool>,
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
        let has_lost = PlayerIndexedVector::new(players_amount);

        Game {
            tick: 1,
            players,
            has_lost,
            params,
            field,
        }
    }

    pub fn has_lost(&self, i: PlayerId) -> bool {
        self.has_lost[i]
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

        let mut loses_in_this_tick = PlayerIndexedVector::new(self.players.len());

        // This phase we sift all the players that are out of borders
        // and collect info about players that collide head to head.
        let mut cell_to_contenders = HashMap::<Cell, Vec<PlayerId>>::new();
        for (player_id, &next_position) in next_position.iter() {
            if self.has_lost[player_id] {
                continue;
            }

            if !next_position.in_bounds() {
                loses_in_this_tick[player_id] = true;
            } else {
                cell_to_contenders
                    .entry(next_position)
                    .or_default()
                    .push(player_id);
            }
        }

        // This phase we process head to head collisions.
        // If two or more players collide and one of them owns this cell, the owner wins.
        // Otherwise, player with shortest tail wins.
        // If multiple players have shortest tail, all of them lose.
        for (&pos, players) in cell_to_contenders.iter() {
            if players.len() <= 1 {
                continue;
            }

            let mut cell_owner = None;
            let mut shortest_path = usize::MAX;
            let mut player_with_shortest_path = players[0];
            let mut multiple_shortest = false;
            for &player_id in players {
                if self.field[pos].is_captured_by(player_id) {
                    cell_owner = Some(player_id);
                }

                let player_path_len = self.field.traced_cells(player_id).len();
                match player_path_len.cmp(&shortest_path) {
                    Ordering::Less => {
                        multiple_shortest = false;
                        shortest_path = player_path_len;
                        player_with_shortest_path = player_id;
                    }
                    Ordering::Equal => {
                        multiple_shortest = true;
                    }
                    Ordering::Greater => {}
                }
            }

            let winner = cell_owner.or(if multiple_shortest {
                None
            } else {
                Some(player_with_shortest_path)
            });

            for &player_id in players {
                if winner != Some(player_id) {
                    loses_in_this_tick[player_id] = true
                }
            }
        }

        // This phase we process players, that capture territory.
        // That is they step into their territory.
        // If player moves within his territory, nothing happens.
        let player_positions = self.players.map(|p| p.position);
        for (player_id, player) in self.players.iter_mut() {
            if loses_in_this_tick[player_id] || self.has_lost[player_id] {
                continue;
            }

            let cell_state = &self.field[next_position[player_id]];
            if cell_state.is_captured_by(player_id) {
                let (enemy_cells_captured, free_cells_captured, enemies_captured) =
                    self.field.capture_all(player_id, &player_positions);

                player.score += enemy_cells_captured * 5 + free_cells_captured;

                for &enemy_id in &enemies_captured {
                    loses_in_this_tick[enemy_id] = true;
                }
            }
        }

        // This phase we process crossing players traces.
        // If two players cross each other at the same time, then the shortest trace wins.
        // If players have traces of the same length, then both of them lose.
        for (my_id, _) in self.players.iter_mut() {
            if self.has_lost[my_id] {
                continue;
            }

            let my_cell_state = self.field[next_position[my_id]];
            if let Some(other_id) = my_cell_state.is_traced() {
                if other_id == my_id {
                    // Self cross.
                    loses_in_this_tick[my_id] = true;
                }

                // We cross someones path, chech if he crosses our path.
                let other_cell_state = self.field[next_position[other_id]];
                let losers: &[_] = if other_cell_state.is_traced_by(my_id) {
                    // We cross each other, shorter path wins.
                    let my_trace_len = self.field.traced_cells(my_id).len();
                    let other_trace_len = self.field.traced_cells(my_id).len();
                    match my_trace_len.cmp(&other_trace_len) {
                        Ordering::Less => &[my_id],
                        Ordering::Equal => &[my_id, other_id],
                        Ordering::Greater => &[other_id],
                    }
                } else {
                    // He does not crosses us, but we cross him.
                    &[other_id]
                };
                for &loser_id in losers {
                    loses_in_this_tick[loser_id] = true;
                }
            }
        }

        // This phase we move players and set their traces.
        for (player_id, player) in self.players.iter_mut() {
            if loses_in_this_tick[player_id] || self.has_lost[player_id] {
                continue;
            }

            let cell_state = self.field[next_position[player_id]];
            if !cell_state.is_captured_by(player_id) {
                self.field.set_trace(next_position[player_id], player_id)
            }
            player.position = next_position[player_id];
        }

        // This phase we marks player that have lost in this tick.
        for (player_id, has_lost) in self.has_lost.iter_mut() {
            if loses_in_this_tick[player_id] {
                self.field.remove_player(player_id);
                *has_lost = true;
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
