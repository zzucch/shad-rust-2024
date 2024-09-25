pub mod traits;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use std::{collections::HashMap, ops::Add};

////////////////////////////////////////////////////////////////////////////////

pub const MAP_SIZE_CELLS: i32 = 31;

////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(tag = "type", content = "params", rename_all = "snake_case")]
pub enum Message {
    StartGame(GameParams),
    Tick(World),
    EndGame {},
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub struct GameParams {
    pub x_cells_count: u32,
    pub y_cells_count: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct World {
    pub players: HashMap<PlayerId, Player>,
    pub tick_num: u32,
}

pub type PlayerId = String;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Player {
    pub score: u32,
    pub territory: Vec<Cell>,
    pub position: Cell,
    pub lines: Vec<Cell>,
    pub direction: Option<Direction>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, FromPrimitive, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up = 0,
    Right,
    Down,
    Left,
}

#[derive(Serialize, Deserialize)]
pub enum Command {
    ChangeDirection(Direction),
    NoOp,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Cell(pub i32, pub i32);

////////////////////////////////////////////////////////////////////////////////

impl World {
    pub fn me(&self) -> &Player {
        self.players.get("i").unwrap()
    }

    pub fn iter_enemies(&self) -> impl Iterator<Item = (&PlayerId, &Player)> {
        self.players.iter().filter_map(|(player_id, player)| {
            if player_id != "i" {
                Some((player_id, player))
            } else {
                None
            }
        })
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = Cell> {
        (0..MAP_SIZE_CELLS).flat_map(|x| (0..MAP_SIZE_CELLS).map(move |y| Cell(x, y)))
    }
}

impl Direction {
    pub fn next(self, clockwise: bool) -> Direction {
        let delta = if clockwise { 1 } else { -1 };
        Self::from_i32((self as i32 + delta + 4) % 4).unwrap()
    }

    pub fn opposite(self) -> Direction {
        Self::from_i32((self as i32 + 2) % 4).unwrap()
    }
}

impl Cell {
    pub fn distance_to(self, other: Cell) -> i32 {
        (other.0 - self.0).abs() + (other.1 - self.1).abs()
    }

    pub fn direction_to(self, other: Cell) -> Direction {
        let (dx, dy) = (other.0 - self.0, other.1 - self.1);
        if dx.abs() > dy.abs() {
            if dx > 0 {
                Direction::Right
            } else {
                Direction::Left
            }
        } else if dy > 0 {
            Direction::Up
        } else {
            Direction::Down
        }
    }

    pub fn iter_neighbours_unchecked(self) -> impl Iterator<Item = Cell> {
        [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .into_iter()
            .map(move |(dx, dy)| Cell(self.0 + dx, self.1 + dy))
    }

    pub fn iter_neighbors(self) -> impl Iterator<Item = Cell> {
        self.iter_neighbours_unchecked().filter(|c| c.in_bounds())
    }

    pub fn adjacent_unchecked(self, dir: Direction) -> Cell {
        match dir {
            Direction::Down => Cell(self.0, self.1 - 1),
            Direction::Up => Cell(self.0, self.1 + 1),
            Direction::Left => Cell(self.0 - 1, self.1),
            Direction::Right => Cell(self.0 + 1, self.1),
        }
    }

    pub fn adjacent(self, dir: Direction) -> Option<Cell> {
        let cell = self.adjacent_unchecked(dir);
        if cell.in_bounds() {
            Some(cell)
        } else {
            None
        }
    }

    pub fn in_bounds(self) -> bool {
        self.0 >= 0 && self.0 < MAP_SIZE_CELLS && self.1 >= 0 && self.1 < MAP_SIZE_CELLS
    }
}

impl Add<Direction> for Cell {
    type Output = Cell;

    fn add(mut self, direction: Direction) -> Self::Output {
        match direction {
            Direction::Up => self.1 += 1,
            Direction::Right => self.0 += 1,
            Direction::Down => self.1 -= 1,
            Direction::Left => self.0 -= 1,
        };
        self
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_test() {
        let start_game = serde_json::from_str::<Message>(
            r#"{
                "type": "start_game",
                "params": {
                    "x_cells_count": 345,
                    "y_cells_count": 567
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            start_game,
            Message::StartGame(GameParams {
                x_cells_count: 345,
                y_cells_count: 567,
            })
        );

        let tick = serde_json::from_str::<Message>(
            r#"{
                "type": "tick",
                "params": {
                    "players": {
                        "1": {
                            "score": 123,
                            "territory": [[0, 0], [0, 1]],
                            "position": [0, 1],
                            "lines": [[1, 0], [1, 1]],
                            "direction": "left"
                        }
                    },
                    "tick_num": 748
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            tick,
            Message::Tick(World {
                players: vec![(
                    "1".to_string(),
                    Player {
                        score: 123,
                        territory: vec![Cell(0, 0), Cell(0, 1)],
                        position: Cell(0, 1),
                        lines: vec![Cell(1, 0), Cell(1, 1)],
                        direction: Some(Direction::Left),
                    }
                )]
                .into_iter()
                .collect(),
                tick_num: 748,
            })
        );

        let end_game =
            serde_json::from_str::<Message>("{\"type\": \"end_game\", \"params\": {}}").unwrap();
        assert_eq!(end_game, Message::EndGame {});
    }
}
