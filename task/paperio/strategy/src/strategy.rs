use paperio_proto::{Cell, Direction, World};
use std::cmp::{max, min};

////////////////////////////////////////////////////////////////////////////////

pub struct Strategy {
    previous_direction: Direction,
    best_rectangle: Option<Rectangle>,
    continuous_useless_ticks: i32,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy {
    pub fn new() -> Self {
        Self {
            previous_direction: Direction::Left,
            best_rectangle: None,
            continuous_useless_ticks: 0,
        }
    }

    pub fn on_tick(&mut self, world: World) -> Direction {
        let me = world.me();

        let mut next_direction: Direction;

        let contains = me.territory.contains(&me.position);
        if contains {
            self.continuous_useless_ticks += 1
        } else {
            self.continuous_useless_ticks = 0
        }

        let new_best_rectangle = match &self.best_rectangle {
            Some(best_rectangle) => {
                contains
                    && ((self.continuous_useless_ticks < 3)
                        || best_rectangle.is_inside(&me.territory))
            }
            None => true,
        };

        if new_best_rectangle {
            let best_cell = world
                .iter_cells()
                .map(|cell| (cell, Self::get_score(&world, &cell)))
                .max_by_key(|x| x.1)
                .map(|x| x.0)
                .unwrap_or(me.position);

            self.best_rectangle = Some(Rectangle::new(&me.position, &best_cell));

            let (dx, dy) = (best_cell.0 - me.position.0, best_cell.1 - me.position.1);
            next_direction = Self::determine_direction(dx, dy, self.previous_direction);
        } else {
            match &self.best_rectangle {
                Some(best_rectangle) => {
                    let adjacent = me.position.adjacent(self.previous_direction);
                    if let Some(adj) = adjacent {
                        if best_rectangle.is_on_perimeter(&adj) {
                            return self.previous_direction;
                        }
                    }

                    next_direction = self.previous_direction.next(true);

                    let adjacent = me.position.adjacent(next_direction);
                    if let Some(adj) = adjacent {
                        if best_rectangle.is_on_perimeter(&adj) {
                            self.previous_direction = next_direction;
                            return next_direction;
                        }
                    }

                    next_direction = next_direction.opposite();
                }
                None => {
                    next_direction = self.previous_direction.next(true);
                }
            }
        }

        self.previous_direction = next_direction;
        next_direction
    }

    fn determine_direction(dx: i32, dy: i32, previous_direction: Direction) -> Direction {
        if dx < 0 && previous_direction != Direction::Right {
            return Direction::Left;
        }

        if dy < 0 && previous_direction != Direction::Up {
            return Direction::Down;
        }

        if dy > 0 && previous_direction != Direction::Down {
            return Direction::Up;
        }

        if dx > 0 && previous_direction != Direction::Left {
            return Direction::Right;
        }

        previous_direction
    }

    fn get_score(world: &World, cell: &Cell) -> i32 {
        let rectangle = Rectangle::new(&world.me().position, cell);

        let cells_score = Self::get_cells_score(world, &rectangle);
        let danger = Self::get_danger_punishment(world, &rectangle);
        let elimination_bonus = Self::get_elimination_bonus(world, &rectangle);
        let save_punishment = if rectangle.is_inside(&world.me().territory) {
            100 * rectangle.get_perimeter()
        } else {
            0
        };

        let bonus = 3 * cells_score + elimination_bonus;
        let punishment = i32::pow(danger, 2) + save_punishment;

        bonus - punishment
    }

    fn get_cells_score(world: &World, rectange: &Rectangle) -> i32 {
        let enemy_area = world
            .iter_cells()
            .filter(|cell| rectange.has_inside(cell))
            .fold(0, |acc, cell| {
                if world.iter_enemies().any(|enemy| {
                    enemy
                        .1
                        .territory
                        .iter()
                        .any(|enemy_cell| enemy_cell.eq(&cell))
                }) {
                    acc + 1
                } else {
                    acc
                }
            });

        enemy_area * 5 + (rectange.get_area() - enemy_area)
    }

    fn get_danger_punishment(world: &World, rectange: &Rectangle) -> i32 {
        let min_enemy_distance = world
            .iter_enemies()
            .map(|enemy| rectange.get_distance(&enemy.1.position))
            .min()
            .unwrap();

        rectange.get_perimeter() - min_enemy_distance
    }

    fn get_elimination_bonus(world: &World, rectangle: &Rectangle) -> i32 {
        let mut factor = 0;

        for enemy in world.iter_enemies() {
            for cell in &enemy.1.lines {
                if rectangle.is_on_perimeter(cell) {
                    let score = enemy.1.score;
                    factor += <u32 as TryInto<i32>>::try_into(score).unwrap();
                }
            }
        }

        factor
    }
}

struct Rectangle {
    corner_1_x: i32,
    corner_1_y: i32,
    corner_2_x: i32,
    corner_2_y: i32,
}

impl Rectangle {
    pub fn new(corner_1: &Cell, corner_2: &Cell) -> Self {
        Self {
            corner_1_x: min(corner_1.0, corner_2.0),
            corner_1_y: min(corner_1.1, corner_2.1),
            corner_2_x: max(corner_1.0, corner_2.0),
            corner_2_y: max(corner_1.1, corner_2.1),
        }
    }

    fn get_width(&self) -> i32 {
        self.corner_2_x - self.corner_1_x
    }

    fn get_height(&self) -> i32 {
        self.corner_2_y - self.corner_1_y
    }

    fn get_area(&self) -> i32 {
        self.get_width() * self.get_height()
    }

    fn get_perimeter(&self) -> i32 {
        2 * (self.get_width() + self.get_height())
    }

    fn has_inside(&self, cell: &Cell) -> bool {
        let (x, y) = (cell.0, cell.1);

        let inside_x = x >= self.corner_1_x && x <= self.corner_2_x;
        let inside_y = y >= self.corner_1_y && y <= self.corner_2_y;

        inside_x && inside_y
    }

    fn get_distance(&self, cell: &Cell) -> i32 {
        let (x, y) = (cell.0, cell.1);

        let mut min_distance = i32::MAX;

        for i in self.corner_1_x..=self.corner_2_x {
            let distance = (i - x).abs() + (self.corner_1_y - y).abs();
            min_distance = min(min_distance, distance);

            let distance = (i - x).abs() + (self.corner_2_y - y).abs();
            min_distance = min(min_distance, distance);
        }

        for j in self.corner_1_y..=self.corner_2_y {
            let distance = (self.corner_1_x - x).abs() + (j - y).abs();
            min_distance = min(min_distance, distance);

            let distance = (self.corner_2_x - x).abs() + (j - y).abs();
            min_distance = min(min_distance, distance);
        }

        min_distance
    }

    fn is_on_perimeter(&self, cell: &Cell) -> bool {
        let (x, y) = (cell.0, cell.1);

        let (a_x, a_y, b_x, b_y) = (
            self.corner_1_x,
            self.corner_1_y,
            self.corner_2_x,
            self.corner_2_y,
        );

        let on_horizontal = (a_x == x || b_x == x) && a_y <= y && y <= b_y;
        let on_vertical = (a_y == y || b_y == y) && a_x <= x && x <= b_x;

        on_horizontal || on_vertical
    }

    fn is_inside(&self, territory: &[Cell]) -> bool {
        for x in self.corner_1_x..=self.corner_2_x {
            for y in self.corner_1_y..=self.corner_2_y {
                let cell = Cell(x, y);
                if !territory.contains(&cell) {
                    return false;
                }
            }
        }

        true
    }
}
