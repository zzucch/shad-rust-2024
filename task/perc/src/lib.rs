use std::vec;

use rand::Rng;

/// Represents a grid of boolean values.
pub struct BoolGrid {
    width: usize,
    height: usize,
    lattice: Vec<Vec<bool>>,
}

impl BoolGrid {
    /// Creates a new grid with all values initialized as `false`.
    ///
    /// # Arguments
    ///
    /// * `width` - grid width.
    /// * `height` - grid height.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            lattice: vec![vec![false; height]; width],
        }
    }

    /// Creates a new grid with every value initialized randomly.
    ///
    /// # Arguments
    ///
    /// * `width` - grid width.
    /// * `height` - grid height.
    /// * `vacancy` - probability of any given value being equal
    ///   to `false`.
    pub fn random(width: usize, height: usize, vacancy: f64) -> Self {
        let mut grid = BoolGrid::new(width, height);

        let mut rng = rand::thread_rng();

        for x in 0..width {
            for y in 0..height {
                if rng.gen_range(0.0..1.0) > vacancy {
                    grid.lattice[x][y] = true;
                }
            }
        }

        grid
    }

    /// Returns grid width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns grid height.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the current value of a given cell.
    /// The caller must ensure that `x` and `y` are valid.
    ///
    /// # Arguments
    ///
    /// * `x` - must be >= 0 and < grid width.
    /// * `y` - must be >= 0 and < grid height.
    ///
    /// # Panics
    ///
    /// If `x` or `y` is out of bounds, this method may panic
    /// (or return incorrect result).
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.lattice[x][y]
    }

    /// Sets a new value to a given cell.
    /// The caller must ensure that `x` and `y` are valid.
    ///
    /// # Arguments
    ///
    /// * `x` - must be >= 0 and < grid width.
    /// * `y` - must be >= 0 and < grid height.
    ///
    /// # Panics
    ///
    /// If `x` or `y` is out of bounds, this method may panic
    /// (or set value to some other unspecified cell).
    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.lattice[x][y] = value
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Returns `true` if the given grid percolates. That is, if there is a path
/// from any cell with `y` == 0 to any cell with `y` == `height` - 1.
/// If the grid is empty (`width` == 0 or `height` == 0), it percolates.
pub fn percolates(grid: &BoolGrid) -> bool {
    if grid.width() == 0 || grid.height() == 0 {
        return true;
    }

    for x in 0..grid.width() {
        let mut visited = vec![vec![false; grid.width()]; grid.height()];

        if dfs(grid, &mut visited, x, 0) {
            return true;
        }
    }

    false
}

pub fn dfs(grid: &BoolGrid, visited: &mut Vec<Vec<bool>>, x: usize, y: usize) -> bool {
    if grid.get(x, y) {
        return false;
    } else if y == grid.height() - 1 {
        return true;
    }

    visited[y][x] = true;

    let moves = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    for (dy, dx) in moves.iter() {
        let y = y.wrapping_add(*dy as usize);
        let x = x.wrapping_add(*dx as usize);

        if y < grid.height() && x < grid.width() && !visited[y][x] && dfs(grid, visited, x, y) {
            return true;
        }
    }

    false
}

////////////////////////////////////////////////////////////////////////////////

const N_TRIALS: u64 = 10000;

/// Returns an estimate of the probability that a random grid with given
/// `width, `height` and `vacancy` probability percolates.
/// To compute an estimate, it runs `N_TRIALS` of random experiments,
/// in each creating a random grid and checking if it percolates.
pub fn evaluate_probability(width: usize, height: usize, vacancy: f64) -> f64 {
    let mut perc_count = 0;
    for _ in 0..N_TRIALS {
        let grid = BoolGrid::random(width, height, vacancy);
        if percolates(&grid) {
            perc_count += 1;
        }
    }
    perc_count as f64 / N_TRIALS as f64
}
