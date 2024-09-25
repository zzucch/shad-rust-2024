use paperio_proto::{Cell, GameParams, PlayerId, World};

#[derive(Debug, Clone)]
pub enum CellState {
    Free,
    Captured(PlayerId),
    Trace(PlayerId),
}

pub struct GameState {
    pub params: GameParams,
    pub field: Vec<Vec<CellState>>,
    pub world: World,
}

impl GameState {
    pub fn new(params: GameParams) -> Self {
        let cells = vec![
            vec![CellState::Free; params.x_cells_count as usize];
            params.y_cells_count as usize
        ];
        Self {
            params,
            field: cells,
            world: World {
                players: Default::default(),
                tick_num: 0,
            },
        }
    }

    fn clear_field(&mut self) {
        for row in &mut self.field {
            for cell in row {
                *cell = CellState::Free;
            }
        }
    }

    pub fn update(&mut self, world: World) {
        self.clear_field();
        for (id, p) in world.players.iter() {
            for &Cell(x, y) in p.territory.iter() {
                self.field[y as usize][x as usize] = CellState::Captured(id.clone());
            }
        }
        for (id, p) in world.players.iter() {
            for &Cell(x, y) in p.lines.iter() {
                self.field[y as usize][x as usize] = CellState::Trace(id.clone());
            }
        }
        self.world = world;
    }
}
