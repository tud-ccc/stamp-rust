use crate::types::Point;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::SplitWhitespace;

pub fn parse_file<P: AsRef<Path>>(path: P) -> (Point, Vec<(Point, Point)>) {
    // FIXME: Error handling for IO, dimension/path check
    let f = File::open(path).unwrap();
    let f = BufReader::new(f);

    let mut dimensions = Point { x: 0, y: 0, z: 0 };
    let mut paths = Vec::new();

    for line in f.lines() {
        let l = line.unwrap().to_lowercase();
        let mut it = l.split_whitespace();

        match it.next() {
            Some("#") => continue,
            Some("d") => dimensions = take_point(&mut it),
            Some("p") => {
                let point1 = take_point(&mut it);
                let point2 = take_point(&mut it);
                paths.push((point1, point2));
            }
            Some(x) => panic!(
                "Encountered malformed input file: Invalid token {} at line start.",
                x
            ),
            None => continue,
        }
    }

    (dimensions, paths)
}

fn take_point(it: &mut SplitWhitespace) -> Point {
    // here comes the good code
    Point {
        x: it.next().unwrap().parse::<usize>().unwrap(),
        y: it.next().unwrap().parse::<usize>().unwrap(),
        z: it.next().unwrap().parse::<usize>().unwrap(),
    }
}
