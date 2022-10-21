#[allow(unused_imports)]
use crate::types::{at_grid_coordinates, Field, Grid, Maze, Path, Point};
#[cfg(feature = "transactional")]
use crate::stm_grid::StmGrid;
#[cfg(all(feature = "transactional", feature = "naive"))]
use stm::{Transaction, StmResult};
#[cfg(all(feature = "transactional", feature = "naive"))]
use crate::types::at_stm_grid_coordinates;
use std::collections::{HashMap, LinkedList};

/// This HashMap contains the information on how to get back from the end point to the start.
/// Each point is assigned the previous point in the path to allow easy backtracking.
#[cfg(feature = "naive")]
type BacktrackMetaData = HashMap<Point, Option<Point>>;

/// This data structure contains information on whether a point has been visited before and
/// information on how to get back from the end point to the start.
/// Each point is assigned the previous point in the path to allow easy backtracking.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum PointStatus {
    #[default]
    Unvisited,
    BacktrackInfo(Option<Point>)
}

impl PointStatus {
    #[inline]
    pub fn unvisited(&self) -> bool {
        *self == Self::Unvisited
    }
}


#[cfg(all(feature = "transactional", feature = "naive"))]
pub fn find_path(points: (Point, Point), grid: &StmGrid, transaction: &mut Transaction) -> StmResult<Option<Path>> {
    compile_error!("Needs to be adjusted to the new `enqueued` Data Structure used in the other `find_path` implementations");
    // TODO: Add costs?
    let (start, end) = points;

    // check if the route is still available
    if at_stm_grid_coordinates(grid, &start, transaction)? != Field::Free {
        return Ok(None);
    }

    let mut unseen_points = VecDeque::new();
    unseen_points.push_back(start.clone());
    let mut visited_points = HashSet::new();
    // the meta_info map contains the backtrack-information for the path
    let mut meta_info: BacktrackMetaData = HashMap::new();
    meta_info.insert(start, None);

    while !unseen_points.is_empty() {
        let current = unseen_points.pop_front().unwrap();

        // stop when reacing the end node
        if current == end {
            return Ok(Some(generate_path(current, meta_info)));
        }

        // get a list of all possible successors
        for child in get_successors(&current, grid) {
            // sort out anything that has been seen or is blocked
            match at_stm_grid_coordinates(grid, &child, transaction)? {
                Field::Used => continue,
                Field::Wall => continue,
                Field::Free => (),
            }

            if visited_points.contains(&child) {
                continue;
            }

            if !unseen_points.contains(&child) {
                meta_info.insert(child.clone(), Some(current.clone()));
                unseen_points.push_back(child);
            }
        }

        visited_points.insert(current);
    }

    // All points have been processed and no path was found
    Ok(None)
}


#[cfg(not(feature = "naive"))]
pub fn find_path(points: (Point, Point), grid: &Grid) -> Option<Path> {
    // TODO: Add costs?
    let (start, end) = points;

    // check if the route is still available
    if at_grid_coordinates(grid, &start) != &Field::Free {
        return None;
    }

    let mut point_status = vec![vec![vec![PointStatus::Unvisited; grid[0][0].len()]; grid[0].len()]; grid.len()];
    let mut unseen_points = LinkedList::new();

    // set the start point
    point_status[start.x][start.y][start.z] = PointStatus::BacktrackInfo(None);
    unseen_points.push_back(start);

    while !unseen_points.is_empty() {
        let current = unseen_points.pop_front().unwrap();

        // stop when reacing the end node
        if current == end {
            return Some(generate_path(current, &point_status));
        }

        // get a list of all possible successors
        for child in get_successors(&current, grid) {
            // sort out anything that has been seen or is blocked
            match at_grid_coordinates(grid, &child) {
                &Field::Used => continue,
                &Field::Wall => continue,
                &Field::Free => (),
            }

            if point_status[child.x][child.y][child.z].unvisited() {
                point_status[child.x][child.y][child.z] = PointStatus::BacktrackInfo(Some(current));
                unseen_points.push_back(child);
            }
        }
    }

    // All points have been processed and no path was found
    None
}

#[cfg(not(feature = "naive"))]
fn get_successors(cur: &Point, grid: &Grid) -> Vec<Point> {
    let mut res = Vec::with_capacity(6);

    if cur.x > 0 {
        res.push(Point {
            x: cur.x - 1,
            y: cur.y,
            z: cur.z,
        });
    }
    if cur.x < grid.len() - 1 {
        res.push(Point {
            x: cur.x + 1,
            y: cur.y,
            z: cur.z,
        });
    }
    if cur.y > 0 {
        res.push(Point {
            x: cur.x,
            y: cur.y - 1,
            z: cur.z,
        });
    }
    if cur.y < grid[0].len() - 1 {
        res.push(Point {
            x: cur.x,
            y: cur.y + 1,
            z: cur.z,
        });
    }
    if cur.z > 0 {
        res.push(Point {
            x: cur.x,
            y: cur.y,
            z: cur.z - 1,
        });
    }
    if cur.z < grid[0][0].len() - 1 {
        res.push(Point {
            x: cur.x,
            y: cur.y,
            z: cur.z + 1,
        });
    }

    res
}

#[cfg(all(feature = "transactional", feature = "naive"))]
fn get_successors(cur: &Point, grid: &StmGrid) -> Vec<Point> {
    let mut res = Vec::with_capacity(6);

    if cur.x > 0 {
        res.push(Point {
            x: cur.x - 1,
            y: cur.y,
            z: cur.z,
        });
    }
    if cur.x < grid.len() - 1 {
        res.push(Point {
            x: cur.x + 1,
            y: cur.y,
            z: cur.z,
        });
    }
    if cur.y > 0 {
        res.push(Point {
            x: cur.x,
            y: cur.y - 1,
            z: cur.z,
        });
    }
    if cur.y < grid[0].len() - 1 {
        res.push(Point {
            x: cur.x,
            y: cur.y + 1,
            z: cur.z,
        });
    }
    if cur.z > 0 {
        res.push(Point {
            x: cur.x,
            y: cur.y,
            z: cur.z - 1,
        });
    }
    if cur.z < grid[0][0].len() - 1 {
        res.push(Point {
            x: cur.x,
            y: cur.y,
            z: cur.z + 1,
        });
    }

    res
}

#[cfg(not(feature = "naive"))]
fn generate_path(end_node: Point, meta_info: &Vec<Vec<Vec<PointStatus>>>) -> Path {
    let mut path = vec![end_node];
    let mut current = end_node;

    while let PointStatus::BacktrackInfo(Some(next)) = meta_info[current.x][current.y][current.z] {
        path.push(next);
        current = next;
    }

    // important, we built the vec from end -> start
    path.reverse();

    Path {
        start: current,
        end: end_node,
        path,
    }
}

#[cfg(feature = "naive")]
fn generate_path(end_node: Point, mut meta_info: BacktrackMetaData) -> Path {
    let mut path = vec![end_node.clone()];
    let mut current = end_node.clone();

    loop {
        if let Some(next) = meta_info.remove(&current).unwrap() {
            path.push(next.clone());
            current = next;
        } else {
            break;
        }
    }

    // important, we built the vec from end -> start
    path.reverse();

    Path {
        start: current,
        end: end_node,
        path: path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn simple_pathfinding() {
        let grid = initialize_grid(4, 1, 1, &None);
        let start = Point { x: 0, y: 0, z: 0 };
        let end = Point { x: 3, y: 0, z: 0 };

        let expected_path = Path {
            start: start.clone(),
            end: end.clone(),
            path: vec![
                Point { x: 0, y: 0, z: 0 },
                Point { x: 1, y: 0, z: 0 },
                Point { x: 2, y: 0, z: 0 },
                Point { x: 3, y: 0, z: 0 },
            ],
        };

        let path = find_path((start, end), &grid);

        assert_eq!(path, Some(expected_path));
    }

    #[test]
    fn pathfinding_with_obstacles() {
        // grid structure:
        // ####G
        // S.##.
        // #....
        let obstacles = vec![
            Point { x: 0, y: 0, z: 0 },
            Point { x: 1, y: 0, z: 0 },
            Point { x: 2, y: 0, z: 0 },
            Point { x: 3, y: 0, z: 0 },
            Point { x: 2, y: 1, z: 0 },
            Point { x: 3, y: 1, z: 0 },
            Point { x: 0, y: 2, z: 0 },
        ];

        let grid = initialize_grid(5, 3, 1, &Some(obstacles));
        let start = Point { x: 0, y: 1, z: 0 };
        let end = Point { x: 4, y: 0, z: 0 };

        let expected_path = Path {
            start: start.clone(),
            end: end.clone(),
            path: vec![
                Point { x: 0, y: 1, z: 0 },
                Point { x: 1, y: 1, z: 0 },
                Point { x: 1, y: 2, z: 0 },
                Point { x: 2, y: 2, z: 0 },
                Point { x: 3, y: 2, z: 0 },
                Point { x: 4, y: 2, z: 0 },
                Point { x: 4, y: 1, z: 0 },
                Point { x: 4, y: 0, z: 0 },
            ],
        };

        let path = find_path((start, end), &grid);

        assert_eq!(path, Some(expected_path));
    }

    #[test]
    fn no_path() {
        // grid structure:
        // ####G
        // S.##.
        // #.#..
        let obstacles = vec![
            Point { x: 0, y: 0, z: 0 },
            Point { x: 1, y: 0, z: 0 },
            Point { x: 2, y: 0, z: 0 },
            Point { x: 3, y: 0, z: 0 },
            Point { x: 2, y: 1, z: 0 },
            Point { x: 3, y: 1, z: 0 },
            Point { x: 0, y: 2, z: 0 },
            Point { x: 2, y: 2, z: 0 },
        ];

        let grid = initialize_grid(5, 3, 1, &Some(obstacles));
        let start = Point { x: 0, y: 1, z: 0 };
        let end = Point { x: 4, y: 0, z: 0 };

        let path = find_path((start, end), &grid);

        assert_eq!(path, None);
    }

    #[test]
    fn no_route_through_used_paths() {
        // grid structure: (o -> used)
        // ####G
        // S.##.
        // #.o..
        let obstacles = vec![
            Point { x: 0, y: 0, z: 0 },
            Point { x: 1, y: 0, z: 0 },
            Point { x: 2, y: 0, z: 0 },
            Point { x: 3, y: 0, z: 0 },
            Point { x: 2, y: 1, z: 0 },
            Point { x: 3, y: 1, z: 0 },
            Point { x: 0, y: 2, z: 0 },
            Point { x: 2, y: 2, z: 0 },
        ];

        let mut grid = initialize_grid(5, 3, 1, &Some(obstacles));
        grid[2][2][0] = Field::Used;
        let start = Point { x: 0, y: 1, z: 0 };
        let end = Point { x: 4, y: 0, z: 0 };

        let path = find_path((start, end), &grid);

        assert_eq!(path, None);
    }
}
