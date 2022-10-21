use stm::TVar;
use crate::Value;

#[derive(Clone, Debug)]
pub struct ComputeCentroid {
    pub coordinates: Vec<f32>,
    pub elements_in_centroid: usize,
}

impl ComputeCentroid {
    pub fn new_empty(number_elements: usize, cluster_count: usize) -> Vec<TVar<Self>> {
        let mut res = Vec::with_capacity(cluster_count);

        for _ in 0..cluster_count {
            res.push(
                TVar::new(Self {
                    coordinates: vec![0.0; number_elements],
                    elements_in_centroid: 0,
                })
            );
        }
            
        res
    }

    /// Resets the coordinate vector and element counter to zero.
    pub fn clear(&mut self) {
        self.coordinates.iter_mut().for_each(|coord| *coord = 0f32);
        self.elements_in_centroid = 0;
    }

    /// Adds the values of a coordinate to the centroid
    pub fn add_value(&mut self, val: &Value) {
        for idx in 0..val.values.len() {
            self.coordinates[idx] += val.values[idx];
        }
        self.elements_in_centroid += 1;
    }
}
