use std::{
    collections::HashSet,
    ops::{Index, IndexMut},
};

use crate::{game::PlayerId, player_vec::PlayerIndexedVector};
use paperio_proto::Cell;

#[derive(Default, Copy, Clone, Debug)]
pub struct CellState {
    captured: Option<PlayerId>,
    traced: Option<PlayerId>,
}

impl CellState {
    pub fn is_traced(&self) -> Option<PlayerId> {
        self.traced
    }

    pub fn is_traced_by(&self, player_id: PlayerId) -> bool {
        self.traced.is_some_and(|id| id == player_id)
    }

    pub fn is_captured_by(&self, player_id: PlayerId) -> bool {
        self.captured.is_some_and(|id| id == player_id)
    }
}

struct Array2D<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T: Default + Clone> Array2D<T> {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![Default::default(); width * height],
        }
    }
}

impl<T> Index<Cell> for Array2D<T> {
    type Output = T;

    fn index(&self, Cell(x, y): Cell) -> &Self::Output {
        &self.data[x as usize + y as usize * self.width]
    }
}

impl<T> IndexMut<Cell> for Array2D<T> {
    fn index_mut(&mut self, Cell(x, y): Cell) -> &mut Self::Output {
        &mut self.data[x as usize + y as usize * self.width]
    }
}

pub struct GameField {
    field: Array2D<CellState>,
    captured_cells: PlayerIndexedVector<HashSet<Cell>>,
    traced_cells: PlayerIndexedVector<HashSet<Cell>>,
}

impl Index<Cell> for GameField {
    type Output = CellState;

    fn index(&self, index: Cell) -> &Self::Output {
        &self.field[index]
    }
}

impl GameField {
    pub fn new(width: usize, height: usize, players_amount: usize) -> Self {
        let field = Array2D::new(width, height);
        let players_territory = PlayerIndexedVector::new(players_amount);
        let players_lines = PlayerIndexedVector::new(players_amount);

        Self {
            field,
            captured_cells: players_territory,
            traced_cells: players_lines,
        }
    }

    pub fn traced_cells(&self, player_id: PlayerId) -> &HashSet<Cell> {
        &self.traced_cells[player_id]
    }

    pub fn set_trace(&mut self, c: Cell, player_id: PlayerId) {
        // Unbind prev cell owner if any
        if let Some(prev_player_id) = self.field[c].traced {
            self.traced_cells[prev_player_id].remove(&c);
        }

        // Bind new owner
        self.field[c].traced = Some(player_id);
        self.traced_cells[player_id].insert(c);
    }

    pub fn set_captured(&mut self, c: Cell, player_id: PlayerId) {
        let cell_state = &mut self.field[c];

        // Erace trace if it is ours
        if cell_state.traced.is_some_and(|id| id == player_id) {
            cell_state.traced = None;
            self.traced_cells[player_id].remove(&c);
        }

        // Unbind prev cell owner if any
        if let Some(prev_player_id) = cell_state.captured {
            self.captured_cells[prev_player_id].remove(&c);
        }

        // Bind new owner
        self.captured_cells[player_id].insert(c);
        cell_state.captured = Some(player_id);
    }

    fn find_inner_cells(&self, player_id: PlayerId) -> Vec<Cell> {
        let mut visited = Array2D::<bool>::new(self.field.width, self.field.height);
        for &c in &self.captured_cells[player_id] {
            visited[c] = true
        }
        for &c in &self.traced_cells[player_id] {
            visited[c] = true
        }

        let mut inner_cells = Vec::<Cell>::new();

        let territory_bounds = self.captured_cells[player_id]
            .iter()
            .chain(self.traced_cells[player_id].iter())
            .flat_map(|&c| c.iter_neighbors());
        for c in territory_bounds {
            if visited[c] {
                continue;
            }
            visited[c] = true;

            let mut border_reached = false;
            let inner_cells_initial_len = inner_cells.len();
            let mut queue_index = inner_cells_initial_len;
            inner_cells.push(c);

            while queue_index < inner_cells.len() {
                let c = inner_cells[queue_index];
                queue_index += 1;
                for n in c.iter_neighbours_unchecked() {
                    if n.in_bounds() {
                        if !visited[n] {
                            inner_cells.push(n);
                            visited[n] = true;
                        }
                    } else {
                        border_reached = true;
                    }
                }
            }

            if border_reached {
                // We reached the border, hence this part is not inner and must be dropped
                inner_cells.truncate(inner_cells_initial_len);
            }
        }

        inner_cells
    }

    pub fn capture_all(
        &mut self,
        player_id: PlayerId,
        players_positions: &PlayerIndexedVector<Cell>,
    ) -> (u32, u32, HashSet<PlayerId>) {
        if self.traced_cells[player_id].is_empty() {
            return (0, 0, HashSet::new());
        }

        let mut captured_cells = self.find_inner_cells(player_id);
        captured_cells.extend(self.traced_cells[player_id].drain());

        let mut enemy_cells_captured = 0;
        let mut free_cells_captured = 0;
        let mut captured_enemies = HashSet::<PlayerId>::new();

        for &cell in captured_cells.iter() {
            let cell_state = self.field[cell];
            match cell_state.captured {
                Some(id) => {
                    if id != player_id {
                        enemy_cells_captured += 1
                    }
                }
                None => free_cells_captured += 1,
            }
            if let Some(other_player_id) = cell_state.traced {
                if other_player_id != player_id {
                    captured_enemies.insert(other_player_id);
                }
            }
            for (enemy_id, &enemy_pos) in players_positions.iter() {
                if enemy_id != player_id && enemy_pos == cell {
                    captured_enemies.insert(enemy_id);
                }
            }

            self.set_captured(cell, player_id)
        }

        (enemy_cells_captured, free_cells_captured, captured_enemies)
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        for traced_cell in self.traced_cells[player_id].drain() {
            self.field[traced_cell].traced = None;
        }
        for captured_cell in self.captured_cells[player_id].drain() {
            self.field[captured_cell].captured = None;
        }
    }

    pub fn get_for_player(&self, player_id: PlayerId) -> (&HashSet<Cell>, &HashSet<Cell>) {
        (
            &self.captured_cells[player_id],
            &self.traced_cells[player_id],
        )
    }

    pub fn init_player(&mut self, player_id: PlayerId, pos: Cell) {
        let Cell(x, y) = pos;
        for i in (x - 1)..=(x + 1) {
            for j in (y - 1)..=(y + 1) {
                self.set_captured(Cell(i, j), player_id)
            }
        }
    }
}
