//! Playground / joinery demos built on [`crate::paneling`] primitives.
//!
//! Production walls use [`crate::paneling`] and [`crate::arcs`] directly
//! (bedroom rectangles, tower [`crate::arcs::portal_ring`]).

pub mod noisy_rectangular_wall;
pub mod terrain_perimeter_wall;

pub use noisy_rectangular_wall::{NoisyRectangularWall, NoisyRectangularWallParams};
pub use terrain_perimeter_wall::TerrainPerimeterWall;
