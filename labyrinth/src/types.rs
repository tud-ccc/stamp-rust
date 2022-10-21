#[cfg(feature = "transactional")]
pub use crate::stm_grid::*;

#[cfg(not(feature = "transactional"))]
pub use crate::grid::*;

use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// A point in the 3D maze
pub struct Point {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}x{}x{}", self.x, self.y, self.z)
    }
}

/// The central maze data structure
#[derive(Debug)]
#[cfg(not(feature = "transactional"))]
pub struct Maze {
    /// The Grid we are working on
    pub grid: Grid,
    /// If any obstacles have been provided, those are stored here
    pub obstacles: Option<Vec<Point>>,
    /// Paths already mapped into the grid
    pub paths: Vec<Path>,
    /// Paths that could not be mapped
    pub unmappable_paths: Vec<(Point, Point)>,
}

#[derive(Debug)]
#[cfg(feature = "transactional")]
pub struct Maze {
    /// The Grid we are working on
    pub grid: StmGrid,
    /// If any obstacles have been provided, those are stored here
    pub obstacles: Option<Vec<Point>>,
    /// Paths already mapped into the grid
    pub paths: Vec<Path>,
    /// Paths that could not be mapped
    pub unmappable_paths: Vec<(Point, Point)>,
}

impl Maze {
    pub fn new(dimensions: Point, obstacles: Option<Vec<Point>>) -> Self {
        Maze {
            grid: initialize_grid(dimensions.x, dimensions.y, dimensions.z, &obstacles),
            obstacles: obstacles,
            paths: Vec::new(),
            unmappable_paths: Vec::new(),
        }
    }

    /// Validates the maze by checking if every path is mapped and every point only used once
    #[cfg(feature = "transactional")]
    pub fn is_valid(&self) -> bool {
        let mut ctrl_grid: Vec<Vec<Vec<Field>>> = self
            .grid
            .iter()
            .map(|x| {
                x.iter()
                    .map(|y| y.iter().map(|var| var.read_atomic()).collect())
                    .collect()
            })
            .collect();

        for path in &self.paths {
            for pt in &path.path {
                if ctrl_grid[pt.x][pt.y][pt.z] == Field::Used {
                    ctrl_grid[pt.x][pt.y][pt.z] = Field::Free;
                } else {
                    return false;
                }
            }
        }

        true
    }

    #[cfg(not(feature = "transactional"))]
    pub fn is_valid(&self) -> bool {
        let mut ctrl_grid = self.grid.clone();

        for path in &self.paths {
            for pt in &path.path {
                if at_grid_coordinates(&ctrl_grid, &pt) == &Field::Used {
                    ctrl_grid[pt.x][pt.y][pt.z] = Field::Free;
                } else {
                    return false;
                }
            }
        }

        true
    }
}

/// A single path in the maze.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    /// Starting point
    pub start: Point,
    /// Ending point
    pub end: Point,
    /// All points to be visited from start to end
    pub path: Vec<Point>,
}

/// A single field. Can be either free or used or it may be a wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    Free,
    Used,
    Wall,
}
