//! Vivaldi coordinate system implementation, based on the paper:
//! [Vivaldi: A Decentralized Network Coordinate System](https://pdos.csail.mit.edu/papers/vivaldi:sigcomm/paper.pdf)
//! by Frank Dabek, Russ Cox, Frans Kaashoek, Robert Morris
//! This implementation is based on the Go implementation by Hashicorp's [Serf](https://github.com/hashicorp/serf/)
//! ```mermaid
//! graph TB
//!    subgraph "Vivaldi Coordinate System"
//!        Node1[Node A] --> Vec1[Vector Components]
//!        Node1 --> H1[Height]
//!        Node1 --> Adj1[Adjustment]
//!
//!        Node2[Node B] --> Vec2[Vector Components]
//!        Node2 --> H2[Height]
//!        Node2 --> Adj2[Adjustment]
//!
//!        Vec1 & Vec2 --> ED[Euclidean Distance]
//!        H1 & H2 --> HD[Height Distance]
//!        Adj1 & Adj2 --> ADJ[Adjustment Factor]
//!
//!        ED & HD & ADJ --> TD[Total Distance]
//!    end
//!
//!    classDef component fill:#f9f,stroke:#333
//!    classDef calculation fill:#bbf,stroke:#333
//!
//!    class Vec1,Vec2,H1,H2,Adj1,Adj2 component
//!    class ED,HD,ADJ,TD calculation
//!
use jetstream_wireformat::JetStreamWireFormat;
use rand::Rng;
use std::time::Duration;

// Constants
const SECONDS_TO_NANOSECONDS: f64 = 1.0e9;
const ZERO_THRESHOLD: f64 = 1.0e-6;

/// Configuration for the Vivaldi coordinate system
#[derive(Debug, Clone, PartialEq, JetStreamWireFormat)]

pub struct Config {
    /// The dimensionality of the coordinate system
    pub dimensionality: usize,
    /// Maximum error value and default for new coordinates
    pub vivaldi_error_max: f64,
    /// Controls maximum impact of observations on confidence
    pub vivaldi_ce: f64,
    /// Controls maximum impact of observations on coordinates
    pub vivaldi_cc: f64,
    /// Number of samples for adjustment window (0 disables feature)
    pub adjustment_window_size: usize,
    /// Minimum height value (must be positive)
    pub height_min: f64,
    /// Maximum samples retained per node for latency filtering
    pub latency_filter_size: usize,
    /// Tuning factor for gravity effect
    pub gravity_rho: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dimensionality: 8,
            vivaldi_error_max: 1.5,
            vivaldi_ce: 0.25,
            vivaldi_cc: 0.25,
            adjustment_window_size: 20,
            height_min: 10.0e-6,
            latency_filter_size: 3,
            gravity_rho: 150.0,
        }
    }
}

/// A Vivaldi coordinate
#[derive(Debug, Clone, JetStreamWireFormat)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Coordinate {
    /// Euclidean portion of the coordinate (in seconds)
    vec: Vec<f64>,
    /// Confidence in the coordinate (dimensionless)
    error: f64,
    /// Distance offset based on observations (in seconds)
    adjustment: f64,
    /// Distance offset for non-Euclidean effects (in seconds)
    height: f64,
}

/// Errors that can occur when working with coordinates
#[derive(Debug, thiserror::Error)]
pub enum CoordinateError {
    /// The coordinate dimensionality does not match
    #[error("coordinate dimensionality does not match")]
    DimensionalityConflict,
    /// The coordinate contains invalid values
    #[error("coordinate contains invalid values")]
    InvalidValues,
    /// The round trip time is not in a valid range
    #[error("round trip time not in valid range")]
    InvalidRtt,
}

impl Coordinate {
    /// Creates a new coordinate at the origin
    pub fn new(config: &Config) -> Self {
        Self {
            vec: vec![0.0; config.dimensionality],
            error: config.vivaldi_error_max,
            adjustment: 0.0,
            height: config.height_min,
        }
    }

    /// Checks if all components of the coordinate are valid
    pub fn is_valid(&self) -> bool {
        self.vec.iter().all(|&x| Self::component_is_valid(x))
            && Self::component_is_valid(self.error)
            && Self::component_is_valid(self.adjustment)
            && Self::component_is_valid(self.height)
    }

    fn component_is_valid(f: f64) -> bool {
        !f.is_infinite() && !f.is_nan()
    }

    /// Checks dimensional compatibility with another coordinate
    pub fn is_compatible_with(&self, other: &Coordinate) -> bool {
        self.vec.len() == other.vec.len()
    }

    /// Returns the distance between this coordinate and another
    pub fn distance_to(
        &self,
        other: &Coordinate,
    ) -> Result<Duration, CoordinateError> {
        if !self.is_compatible_with(other) {
            return Err(CoordinateError::DimensionalityConflict);
        }

        let dist = self.raw_distance_to(other);
        let adjusted_dist = dist + self.adjustment + other.adjustment;
        let final_dist = if adjusted_dist > 0.0 {
            adjusted_dist
        } else {
            dist
        };

        Ok(Duration::from_nanos(
            (final_dist * SECONDS_TO_NANOSECONDS) as u64,
        ))
    }

    /// Calculates raw Vivaldi distance without adjustments
    fn raw_distance_to(&self, other: &Coordinate) -> f64 {
        let euclidean_dist = self
            .vec
            .iter()
            .zip(other.vec.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();

        euclidean_dist + self.height + other.height
    }

    /// Applies force from another coordinate's direction
    pub fn apply_force(
        &self,
        config: &Config,
        force: f64,
        other: &Coordinate,
    ) -> Result<Coordinate, CoordinateError> {
        if !self.is_compatible_with(other) {
            return Err(CoordinateError::DimensionalityConflict);
        }

        let (unit, mag) = self.unit_vector_at(other);
        let mut new_coord = self.clone();

        // Update vector components
        (0..self.vec.len()).for_each(|i| {
            new_coord.vec[i] += unit[i] * force;
        });

        // Update height if points aren't too close
        if mag > ZERO_THRESHOLD {
            new_coord.height = (new_coord.height + other.height) * force / mag
                + new_coord.height;
            new_coord.height = new_coord.height.max(config.height_min);
        }

        Ok(new_coord)
    }

    /// Calculates unit vector pointing from this coordinate to another
    fn unit_vector_at(&self, other: &Coordinate) -> (Vec<f64>, f64) {
        let mut diff: Vec<f64> = self
            .vec
            .iter()
            .zip(other.vec.iter())
            .map(|(&a, &b)| a - b)
            .collect();

        let mag = (diff.iter().map(|&x| x * x).sum::<f64>()).sqrt();

        // If points aren't too close, normalize the vector
        if mag > ZERO_THRESHOLD {
            for d in diff.iter_mut() {
                *d /= mag;
            }
            return (diff, mag);
        }

        // Generate random unit vector if points are too close
        // SAFETY: this is not used for cryptographic purposes
        // so it's fine to use the default RNG
        let mut rng = rand::thread_rng();
        diff = diff.iter().map(|_| rng.gen::<f64>() - 0.5).collect();
        let mag = (diff.iter().map(|&x| x * x).sum::<f64>()).sqrt();

        if mag > ZERO_THRESHOLD {
            for d in diff.iter_mut() {
                *d /= mag;
            }
            return (diff, 0.0);
        }

        // Last resort: unit vector along first dimension
        let mut unit = vec![0.0; self.vec.len()];
        unit[0] = 1.0;
        (unit, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rand::seq::SliceRandom;

    use super::*;

    fn _verify_equal_vectors(v1: &[f64], v2: &[f64], epsilon: f64) {
        assert_eq!(v1.len(), v2.len());
        for (a, b) in v1.iter().zip(v2.iter()) {
            assert!((a - b).abs() < epsilon, "Expected {a} to equal {b}");
        }
    }

    #[test]
    fn test_new_coordinate() {
        let config = Config::default();
        let coord = Coordinate::new(&config);
        assert_eq!(coord.vec.len(), config.dimensionality);
        assert_eq!(coord.error, config.vivaldi_error_max);
        assert_eq!(coord.height, config.height_min);
    }

    #[test]
    fn test_is_valid() {
        let config = Config::default();
        let mut coord = Coordinate::new(&config);
        assert!(coord.is_valid());

        coord.vec[0] = f64::NAN;
        assert!(!coord.is_valid());

        coord.vec[0] = f64::INFINITY;
        assert!(!coord.is_valid());
    }

    #[test]
    fn test_incompatible_dimensions() {
        let mut config = Config {
            dimensionality: 3,
            ..Default::default()
        };

        let coord1 = Coordinate::new(&config);

        config.dimensionality = 2;
        let coord2 = Coordinate::new(&config);

        assert!(!coord1.is_compatible_with(&coord2));
        assert!(coord1.distance_to(&coord2).is_err());
    }
    #[test]
    fn test_distance_calculation() {
        let config = Config::default();
        let mut coord1 = Coordinate::new(&config);
        let mut coord2 = Coordinate::new(&config);

        // Set first 3 dimensions and pad rest with zeros
        coord1.vec = vec![-0.5, 1.3, 2.4];
        coord2.vec = vec![1.2, -2.3, 3.4];

        while coord1.vec.len() < config.dimensionality {
            coord1.vec.push(0.0);
            coord2.vec.push(0.0);
        }

        // Calculate euclidean distance
        let euclidean_distance = 4.104875150354758;

        // Expected total distance includes heights
        let expected_distance =
            euclidean_distance + coord1.height + coord2.height;

        let dist = coord1.distance_to(&coord2).unwrap();
        let got = dist.as_secs_f64();
        let diff = (got - expected_distance).abs();

        println!("Distance calculation:");
        println!("Got:      {}", got);
        println!("Expected: {}", expected_distance);
        println!("Diff:     {}", diff);

        assert!(
            diff < 1e-6,
            "Distance calculation failed:\nGot {}\nWanted {}\nDiff {}",
            got,
            expected_distance,
            diff
        );

        // Additional verification tests
        let mut height_free_coord1 = coord1.clone();
        let mut height_free_coord2 = coord2.clone();

        // Test with zero heights
        height_free_coord1.height = 0.0;
        height_free_coord2.height = 0.0;

        let pure_euclidean =
            height_free_coord1.distance_to(&height_free_coord2).unwrap();
        assert!(
            (pure_euclidean.as_secs_f64() - euclidean_distance).abs() < 1e-6,
            "Pure Euclidean distance failed:\nGot {}\nWanted {}",
            pure_euclidean.as_secs_f64(),
            euclidean_distance
        );
    }

    // Add test for pure height calculation
    #[test]
    fn test_height_contribution() {
        let config = Config::default();
        let coord1 = Coordinate::new(&config);
        let coord2 = Coordinate::new(&config);

        // With default config, both coords should have height_min
        let expected_height_contribution = config.height_min * 2.0;
        let dist = coord1.distance_to(&coord2).unwrap();

        assert!(
            (dist.as_secs_f64() - expected_height_contribution).abs() < 1e-6,
            "Height-only distance failed:\nGot {}\nWanted {}",
            dist.as_secs_f64(),
            expected_height_contribution
        );
    }

    // Add test for adjustment contribution
    #[test]
    fn test_adjustment_contribution() {
        let config = Config::default();
        let mut coord1 = Coordinate::new(&config);
        let mut coord2 = Coordinate::new(&config);

        coord1.adjustment = 0.1;
        coord2.adjustment = 0.2;

        let base_dist = coord1.height + coord2.height; // Just heights, vectors are 0
        let expected_dist = base_dist + 0.3; // 0.3 is sum of adjustments

        let dist = coord1.distance_to(&coord2).unwrap();

        assert!(
            (dist.as_secs_f64() - expected_dist).abs() < 1e-6,
            "Adjustment distance failed:\nGot {}\nWanted {}",
            dist.as_secs_f64(),
            expected_dist
        );
    }

    // Generate a random map of coordinates with 20 different nodes.
    // This is a simple test to verify that the map is generated correctly.
    #[test]
    fn test_map_generation() {
        let config = Config::default();
        let mut coordinates = vec![Coordinate::new(&config); 20];
        let mut node_map: HashMap<String, Coordinate> = HashMap::new();

        // crates a rand that's deterministic with a seed
        let mut rng = rand::rngs::mock::StepRng::new(2, 10);

        // apply force to each coordinate in relation to another random coordinate
        Iterator::for_each(0..coordinates.len(), |i| {
            let other = coordinates.choose(&mut rng).unwrap();

            let force: f64 = rng.gen_range(0.0..2.0);
            coordinates[i] =
                coordinates[i].apply_force(&config, force, other).unwrap();

            // Insert into the map with "node_{i}" as the key
            node_map.insert(format!("node_{}", i), coordinates[i].clone());
        });
        // sort the map so test doesn't fail due to different ordering
        let mut node_map: Vec<_> = node_map.into_iter().collect();
        node_map.sort_by(|a, b| a.0.cmp(&b.0));
        #[cfg(feature = "serde")]
        {
            insta::assert_json_snapshot!(node_map);
        }
    }
}
