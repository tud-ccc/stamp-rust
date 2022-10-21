use crate::types::*;
use stm::{StmError, StmResult, TVar, Transaction};

/// The grid the maze is set up in. Contains for every field the information about its state.
pub type StmGrid = Vec<Vec<Vec<TVar<Field>>>>;
pub type Grid = Vec<Vec<Vec<Field>>>;

/// Initializes a new, empty grid, optionally with pre-defined walls which can be provided using the `walls` parameter.
///
/// The width defines the domain of the x-axis, the depth the domain of the y axis and depth the z-axis.
pub fn initialize_grid(
    width: usize,
    height: usize,
    depth: usize,
    walls: &Option<Vec<Point>>,
) -> StmGrid {
    let mut grid = vec![vec![vec![TVar::new(Field::Free); depth]; height]; width];

    // must initialize this way because of the `clone` semantics of `vec!`
    for x in 0..width {
        for y in 0..height {
            for z in 0..depth {
                grid[x][y][z] = TVar::new(Field::Free);
            }
        }
    }

    // place walls if any
    if let Some(wall_vec) = walls {
        for wall in wall_vec {
            grid[wall.x][wall.y][wall.z] = TVar::new(Field::Wall);
        }
    }

    grid
}

pub fn at_stm_grid_coordinates<'a>(
    grid: &'a StmGrid,
    pt: &Point,
    transaction: &mut Transaction,
) -> StmResult<Field> {
    Ok(grid[pt.x][pt.y][pt.z].read(transaction)?)
}

pub fn at_grid_coordinates<'a>(grid: &'a Grid, pt: &Point) -> &'a Field {
    &grid[pt.x][pt.y][pt.z]
}

/// Updates the grid with mapped paths.
pub fn update_grid(grid: &StmGrid, path: &Path, transaction: &mut Transaction) -> StmResult<()> {
    for pt in &path.path {
        if at_stm_grid_coordinates(grid, pt, transaction)? == Field::Free {
            grid[pt.x][pt.y][pt.z].write(transaction, Field::Used)?;
        } else {
            return Err(StmError::Retry);
        }
    }

    Ok(())
}

pub fn create_working_copy(grid: &StmGrid) -> Grid {
    grid.iter()
        .map(|y_grid| {
            y_grid
                .iter()
                .map(|z_grid| z_grid.iter().map(|pt| pt.read_atomic()).collect())
                .collect()
        })
        .collect()
}
