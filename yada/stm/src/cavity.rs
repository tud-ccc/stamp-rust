use crate::element::{Edge, Element, Triangle};
use crate::mesh::Mesh;
use crate::point::Point;
use std::collections::VecDeque;
use stm::{StmError, StmResult, Transaction};

/// Connection between two elements. Format: (src, edge, dest)
type Connection = (Element, Edge, Element);

pub struct Cavity {
    pub center_element: Element,
    pub center: Point,

    pub frontier: VecDeque<Element>,
    pub previous_nodes: Vec<Element>,
    pub new_nodes: Vec<Element>,
    pub new_connections: Vec<Connection>,

    /// Connections to elements neighboring the cavity.
    pub connections: Vec<Connection>,

    pub dimension: usize,
}

impl Cavity {
    /// Initializes a new cavity.
    pub fn new(_mesh: &Mesh, node: Element) -> Option<Self> {
        let mut frontier = VecDeque::new();
        let mut previous_nodes = Vec::new();
        let new_nodes = Vec::new();

        let center_element = node;

        // TODO(feliix42): What if mesh.contains(node) fails?
        // let mut circ = Vec::new();
        // while mesh.contains(&center_element) && center_element.has_obtuse() {
        //     if let Element::T(t) = center_element {
        //         circ.push(t.into());
        //         let new_center = mesh.get_opposite(&t);
        //         // the original code did not handle loops at all (i.e. 2 triangles that share an opposite side)
        //         if circ.contains(&new_center) {
        //             break;
        //         } else {
        //             center_element = new_center;
        //         }
        //     } else {
        //         unreachable!()
        //     }
        // }

        let center = center_element.get_center()?;
        let dimension = match center_element {
            Element::T(_) => 3,
            Element::E(_) => 2,
        };
        frontier.push_back(center_element);
        previous_nodes.push(center_element);

        Some(Cavity {
            center_element,
            center,
            frontier,
            previous_nodes,
            new_nodes,
            new_connections: Vec::new(),
            connections: Vec::new(),
            dimension,
        })
    }

    fn expand(&mut self, curr: Element, next: Element) -> Result<(), Element> {
        if !(self.dimension == 2 && next.is_edge() && next != self.center_element)
            && next.in_circle(self.center)
        {
            // part of the cavity and we're not the second segment encroaching on this cavity
            if next.is_edge() && self.dimension != 2 {
                // is segment and we're encroaching
                return Err(next);
            } else if !self.previous_nodes.contains(&next) {
                self.previous_nodes.push(next);
                self.frontier.push_back(next);
            }
        } else {
            // not a member
            let edge = next.get_related_edge(&curr).unwrap();

            let connection = (curr, edge, next);
            if !self.connections.contains(&connection) {
                self.connections.push(connection);
            } else {
            }
        }

        Ok(())
    }

    /// Expand the cavity to cover all related elements.
    pub fn build(&mut self, mesh: &Mesh, trans: &mut Transaction) -> StmResult<()> {
        let elements = mesh.elements.read(trans)?;
        let boundary_set = mesh.boundary_set.read(trans)?;

        // NOTE(feliix42): Due to STMs consistency model, it may or may not happen here that our
        // reads run into the `expect` clause of the hashmap accesses. Should that happen, reformat
        // them into a `return Err(StmError::Failure);` expression.
        while !self.frontier.is_empty() {
            let curr = self.frontier.pop_back().unwrap();

            match curr {
                Element::T(ref t) => {
                    if let Some(neighbors) = elements.get(t) {
                        for neighbor in neighbors.read(trans)? {
                            if let Err(other) = self.expand(curr, neighbor) {
                                *self = Self::new(mesh, other).unwrap();
                            }
                        }
                    } else {
                        //println!("Element is no longer in triangle set");
                        return Err(StmError::Failure);
                    }
                }
                Element::E(ref e) => {
                    if let Some(neighbor) = boundary_set.get(e) {
                        if let Err(other) = self.expand(curr, *neighbor) {
                            *self = Self::new(mesh, other).unwrap();
                        }
                    } else {
                        //println!("Element is no longer in boundary set");
                        return Err(StmError::Failure);
                    }
                }
            }
        }

        Ok(())
    }

    /// Compute a corrected cavity
    pub fn compute(&mut self) {
        // TODO: IS THIS FUNCTION CORRECT??
        if self.dimension == 2 {
            // we've actually built around a segment (or an edge)
            let c = self.center_element.get_points();
            let n1 = Edge::new(self.center, *c[0]);
            let n2 = Edge::new(self.center, *c[1]);

            self.new_nodes.push(n1.into());
            self.new_nodes.push(n2.into());
        }

        //let mut circtest = Vec::new();
        for conn in &self.connections {
            // cycle detection (temporary)
            //if circtest.contains(conn) {
            //panic!("ALARM");
            //}
            //circtest.push(*conn);
            assert_ne!(conn.1 .0, conn.1 .1);

            let ele = Element::T(Triangle::new(self.center, (conn.1).0, (conn.1).1));
            let other = if self.previous_nodes.contains(&conn.2) {
                // if the destination is contained in previous nodes, go for the source
                conn.0
            } else {
                conn.2
            };

            let other_edge = match ele.get_related_edge(&other) {
                Some(e) => e,
                None => panic!(
                    "There is no related edge between the following coordinates:\n{:#?}\n{:#?}\nMatch check: {}\nEquality? {}\nCenter: {:?}",
                    ele.get_points(),
                    other.get_points(),
                    ele.is_related_to(&other),
                    ele == other,
                    self.center
                ),
            };
            // Connection structure: (source, shared edge, destination)
            self.new_connections.push((ele, other_edge, other));

            for element in &self.new_nodes {
                if element.is_related_to(&ele) {
                    let additional_edge = ele.get_related_edge(element).unwrap();
                    self.new_connections.push((ele, additional_edge, *element));
                }
            }

            self.new_nodes.push(ele);
        }
    }
}
