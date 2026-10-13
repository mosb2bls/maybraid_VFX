//! Maputo: furnishing over a ground that exposes [`FurnitureSlots`].
//!
//! Furniture cells generate on the shared HCSG runtime and present as flattened
//! hosts around the viewer.

mod cell;
mod colliders;
pub mod hcsg;
mod host;
mod slots;

pub use cell::{
	FurnitureCellExtent, FURNITURE_CELL_SIZE, FURNITURE_GENERATE_RADIUS, FURNITURE_PRESENT_RADIUS,
};
pub use colliders::FurnitureWalkCollider;
pub use hcsg::{DevelopmentSlots, Furnished, FurnitureNeighborhood, MaputoPresentationPlugin};
pub use host::{spawn_furniture_cell, FurnitureCell};
pub use slots::FurnitureSlots;

#[cfg(test)]
mod tests;
