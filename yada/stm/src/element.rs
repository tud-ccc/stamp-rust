#![allow(dead_code)]
//! In the original codebase, this was `element.c`

use crate::point::Point;
use decorum::R64;
use num_traits::real::Real;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Element {
    T(Triangle),
    E(Edge),
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Element::E(ref e) => write!(f, "E: ({}, {}) ({}, {})", e.0.x, e.0.y, e.1.x, e.1.y),
            Element::T(ref t) => t.fmt(f),
        }
    }
}

impl Element {
    /// Determines whether an element needs to be processed.
    pub fn is_bad(&self, min_angle: f64) -> bool {
        match self {
            Self::T(t) => t.is_bad(min_angle),
            Self::E(e) => e.is_bad(),
        }
    }

    /// Get the center point of the element.
    pub fn get_center(&self) -> Option<Point> {
        match self {
            Self::T(t) => t.get_center(),
            Self::E(e) => e.get_center(),
        }
    }

    /// Get the edge with the assigned number.
    pub fn get_edge(&self, i: usize) -> Edge {
        match self {
            Self::T(t) => t.get_edge(i),
            Self::E(e) => e.get_edge(i),
        }
    }

    /// Get the point with the obtuse angle.
    pub fn get_obtuse(&self) -> Point {
        match self {
            Self::T(t) => t.get_obtuse(),
            Self::E(_) => panic!("A line has no obtuse angles."),
        }
    }

    pub fn has_obtuse(&self) -> bool {
        match self {
            Element::T(ref t) => t.obtuse_angle.is_some(),
            Element::E(_) => false,
        }
    }

    fn get_radius(&self, pt: Point) -> R64 {
        match self {
            Self::T(t) => t.get_radius(pt),
            Self::E(e) => e.get_radius(pt),
        }
    }

    /// Returns a list of points the element is composed of.
    pub fn get_points(&self) -> Vec<&Point> {
        match self {
            Self::T(t) => t.get_points(),
            Self::E(e) => e.get_points(),
        }
    }

    pub fn in_circle(&self, p: Point) -> bool {
        if let Some(center) = self.get_center() {
            let ds = center.squared_distance_to(&p);
            ds <= self.get_radius(center)
        } else {
            false
        }
    }

    pub fn is_triangle(&self) -> bool {
        matches!(self, Self::T(_))
    }

    pub fn is_edge(&self) -> bool {
        matches!(self, Self::E(_))
    }

    /// Returns `true` if both elements share an edge.
    pub fn is_related_to(&self, other: &Element) -> bool {
        let this_pt = self.get_points();
        let other_pt = other.get_points();

        let mut num_matching_points = 0;

        for pt in this_pt {
            for pt2 in &other_pt {
                if &pt == pt2 {
                    num_matching_points += 1;
                }
            }
        }

        num_matching_points == 2
    }

    /// If both elements share an edge, it is returned.
    pub fn get_related_edge(&self, other: &Element) -> Option<Edge> {
        let this_pt = self.get_points();
        let other_pt = other.get_points();
        let mut points: Vec<Point> = Vec::with_capacity(2);

        for coord in this_pt {
            for ocoord in &other_pt {
                if &coord == ocoord {
                    points.push(*coord);
                }
            }
        }

        if points.len() == 2 {
            Some(Edge::new(points[0], points[1]))
        } else {
            panic!("Found no related edge, got: {:?}", points);
            //None
        }
    }
}

impl From<Triangle> for Element {
    fn from(t: Triangle) -> Self {
        Self::T(t)
    }
}

impl From<Edge> for Element {
    fn from(e: Edge) -> Self {
        Self::E(e)
    }
}

// pub trait Element {
//     /// Determines whether an element needs to be processed.
//     fn is_bad(&self) -> bool;
//     /// Get a list of (references to) all points of the element.
//     fn get_points<'a>(&'a self) -> Vec<&'a Point>;
//     /// Get the center point of the element.
//     fn get_center(&self) -> Point;
//     /// Get the edge with the assigned number.
//     fn get_edge(&self, i: usize) -> Edge;
//     /// Get the point with the obtuse angle.
//     fn get_obtuse(&self) -> Point;
//     /// Returns `true` if both elements share an edge.
//     fn is_related_to(&self, other: &dyn Element) -> bool {
//         let this_pt = self.get_points();
//         let other_pt = other.get_points();

//         let mut num_matching_points = 0;

//         for pt in this_pt {
//             for pt2 in &other_pt {
//                 if &pt == pt2 {
//                     num_matching_points += 1;
//                 }
//             }
//         }

//         num_matching_points == 2
//     }

//     /// If both elements share an edge, it is returned.
//     fn get_related_edge(&self, other: &dyn Element) -> Option<Edge> {
//         let this_pt = self.get_points();
//         let other_pt = other.get_points();
//         let mut points: Vec<Point> = Vec::with_capacity(2);

//         for coord in this_pt {
//             for ocoord in &other_pt {
//                 if &coord == ocoord {
//                     points.push(*coord);
//                 }
//             }
//         }

//         if points.len() == 2 {
//             Some(Edge::new(points[0], points[1]))
//         } else {
//             panic!("Found no related edge, got: {:?}", points);
//             //None
//         }
//     }

//     fn get_radius(&self, pt: Point) -> f64;

//     fn in_circle(&self, p: Point) -> bool {
//         let center = self.get_center();
//         let ds = center.distance_to(&p);
//         ds <= self.get_radius(center)
//     }
// }

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Triangle {
    /// The coordinates of the triangle, sorted by size.
    coordinates: [Point; 3],

    /// The index of the point that houses the obtuse angle.
    pub obtuse_angle: Option<usize>,
}

impl fmt::Display for Triangle {
    // fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    //     writeln!(
    //         f,
    //         "T: ({}, {})",
    //         self.coordinates[0].x, self.coordinates[0].y
    //     )?;
    //     writeln!(
    //         f,
    //         "   ({}, {})",
    //         self.coordinates[1].x, self.coordinates[1].y
    //     )?;
    //     write!(
    //         f,
    //         "   ({}, {})",
    //         self.coordinates[2].x, self.coordinates[2].y
    //     )
    // }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "T: ({}, {}) ({}, {}) ({}, {})",
            self.coordinates[0].x,
            self.coordinates[0].y,
            self.coordinates[1].x,
            self.coordinates[1].y,
            self.coordinates[2].x,
            self.coordinates[2].y
        )
    }
}

impl Triangle {
    /// Create a new triangle
    pub fn new(p1: Point, p2: Point, p3: Point) -> Self {
        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
        assert_ne!(p3, p1);

        let mut coordinates: [Point; 3] = [p1, p2, p3];
        // coordinates.sort_unstable();
        // this is super counter intuitive but this is how the implementation looks
        if p2 < p1 || p3 < p1 {
            if p2 < p3 {
                coordinates = [p2, p3, p1];
            } else {
                coordinates = [p3, p1, p2];
            }
        }

        // coordinates.sort_unstable();
        let mut obtuse = None;

        for i in 0..3 {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            if coordinates[j].angle_is_obtuse(&coordinates[i], &coordinates[k]) {
                obtuse = Some(i);
            }
        }

        Triangle {
            coordinates,
            obtuse_angle: obtuse,
        }
    }

    pub fn area(&self) -> R64 {
        let a = self.coordinates[0];
        let b = self.coordinates[1];
        let c = self.coordinates[2];

        ((a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) / 2_f64).abs()
    }

    pub fn is_bad(&self, min_angle: f64) -> bool {
        // Stopgap measure
        if self.area() < 1e-1 {
            return false;
        }

        for i in 0..3 {
            // I'm not exactly sure what happens here, it's a verbatim copy of the Galois function
            // But the tests say it works, so....
            if self.coordinates[i].angle_is_greater_than(
                &self.coordinates[(i + 1) % 3],
                &self.coordinates[(i + 2) % 3],
                min_angle,
            ) {
                return true;
            }
            // if let Some(ang) = self.coordinates[i].angle(
            //     &self.coordinates[(i + 1) % 3],
            //     &self.coordinates[(i + 2) % 3],
            // ) {
            //     if ang < MIN_ANGLE {
            //         return true;
            //     }
            // }
        }

        false
    }

    pub fn get_center(&self) -> Option<Point> {
        let a = self.coordinates[0];
        let b = self.coordinates[1];
        let c = self.coordinates[2];

        let x = b - a;
        let y = c - a;
        let x_len = a.distance_to(&b);
        let y_len = a.distance_to(&c);
        let cosine = (x * y) / (x_len * y_len);
        let sine_sq = R64::from_inner(1.0) - cosine * cosine;
        let p_len = y_len / x_len;

        let s = p_len * cosine;
        let t = p_len * sine_sq;
        if t.abs() < R64::epsilon() {
            return None;
        }

        let wp = (p_len - cosine) / (R64::from_inner(2.0) * t);
        let wb = R64::from_inner(0.5) - (wp * s);

        let mut tmp_val = a * (R64::from_inner(1f64) - wb - wp);
        tmp_val = tmp_val + (b * wb);
        Some(tmp_val + (c * wp))
    }

    pub fn get_points(&self) -> Vec<&Point> {
        vec![
            &self.coordinates[0],
            &self.coordinates[1],
            &self.coordinates[2],
        ]
    }

    pub fn get_edge(&self, i: usize) -> Edge {
        match i {
            0 => Edge::new(self.coordinates[0], self.coordinates[1]),
            1 => Edge::new(self.coordinates[1], self.coordinates[2]),
            2 => Edge::new(self.coordinates[2], self.coordinates[0]),
            // error case
            _ => Edge::new(self.coordinates[0], self.coordinates[0]),
        }
    }

    pub fn get_obtuse(&self) -> Point {
        if let Some(i) = self.obtuse_angle {
            self.coordinates[i]
        } else {
            panic!("No obtuse angle exists for this triangle!");
        }
    }

    /// Get the Edge opposite to the obtuse angle.
    pub fn get_opposite_edge(&self) -> Edge {
        if let Some(i) = self.obtuse_angle {
            self.get_edge((i + 1) % 3)
        } else {
            panic!("No obtuse angle exists for this triangle!")
        }
    }

    pub fn get_radius(&self, pt: Point) -> R64 {
        pt.squared_distance_to(&self.coordinates[0])
    }

    /// If both elements share an edge, it is returned.
    pub fn get_related_edge(&self, other: &Triangle) -> Option<Edge> {
        let mut points: Vec<Point> = Vec::with_capacity(2);

        for coord in &self.coordinates {
            for ocoord in &other.coordinates {
                if coord == ocoord {
                    points.push(*coord);
                }
            }
        }

        if points.len() == 2 {
            Some(Edge::new(points[0], points[1]))
        } else {
            panic!("Found no related edge, got: {:?}", points);
            //None
        }
    }
}

#[derive(Copy, Clone, Debug, Eq)]
pub struct Edge(pub Point, pub Point);

impl PartialEq for Edge {
    fn eq(&self, other: &Edge) -> bool {
        // I sure hope this won't come back to bite me
        self.0 == other.0 && self.1 == other.1 || self.0 == other.1 && self.1 == other.0
    }
}

impl Hash for Edge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0 < self.1 {
            self.0.hash(state);
            self.1.hash(state);
        } else {
            self.1.hash(state);
            self.0.hash(state);
        }
    }
}

impl Edge {
    pub fn new(p1: Point, p2: Point) -> Self {
        if p1 < p2 {
            Edge(p1, p2)
        } else {
            Edge(p2, p1)
        }
    }

    pub fn is_bad(&self) -> bool {
        // this can't actually ever be true
        false
    }

    pub fn get_center(&self) -> Option<Point> {
        Some((self.0 + self.1) * 0.5)
    }

    pub fn get_points(&self) -> Vec<&Point> {
        vec![&self.0, &self.1]
    }

    pub fn get_edge(&self, i: usize) -> Edge {
        match i {
            1 => Self(self.1, self.0),
            _ => *self,
        }
    }

    pub fn get_obtuse(&self) -> Point {
        panic!("A line has no obtuse angles.")
    }

    pub fn get_radius(&self, pt: Point) -> R64 {
        pt.squared_distance_to(&self.0)
    }

    pub fn contains(&self, pt: Point) -> bool {
        self.0 == pt || self.1 == pt
    }
}
/*
impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        // everything is derived from the coordinates
        self.coordinates == other.coordinates
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        self.coordinates.cmp(&other.coordinates)
    }
}

*/

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn good_triangles_work() {
        let p0 = Point {
            x: R64::from_inner(0_f64),
            y: R64::from_inner(0_f64),
        };
        let p1 = Point {
            x: R64::from_inner(10_f64),
            y: R64::from_inner(0_f64),
        };
        let p2 = Point {
            x: R64::from_inner(0_f64),
            y: R64::from_inner(10_f64),
        };

        let e = Element::T(Triangle::new(p0, p1, p2));

        assert!(e.is_bad() == false);
    }

    #[test]
    fn bad_triangles_work() {
        let p0 = Point {
            x: R64::from_inner(0_f64),
            y: R64::from_inner(0_f64),
        };
        let p1 = Point {
            x: R64::from_inner(10_f64),
            y: R64::from_inner(0_f64),
        };
        let p2 = Point {
            x: R64::from_inner(0_f64),
            y: R64::from_inner(1000000_f64),
        };

        let e = Element::T(Triangle::new(p0, p1, p2));

        assert!(e.is_bad() == true);
    }

    #[test]
    fn skip_small_triangles() {
        let p0 = Point {
            x: R64::from_inner(92.82096),
            y: R64::from_inner(64.98023),
        };
        let p1 = Point {
            x: R64::from_inner(92.82097),
            y: R64::from_inner(64.98023),
        };
        let p2 = Point {
            x: R64::from_inner(92.82096),
            y: R64::from_inner(64.98024),
        };

        let e = Triangle::new(p0, p1, p2);

        println!("{}", e.area());
        assert!(e.is_bad() == false);
    }
}
