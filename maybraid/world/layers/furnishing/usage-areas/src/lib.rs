//! Expand Richmond [`FurnitureUsageNode`](building_components::FurnitureUsageNode)
//! regions into furniture ensembles.
//!
//! Parallel to [`furniture-assemblies`](https://github.com/ramate-io/maybraid): Richmond
//! packs labels + region AABBs; this crate fills those boxes with kit slots.
//! It depends on [`building-components`] only — not `buildings`.

pub mod bites_counter;
pub mod bites_kitchen;
pub mod bites_seating;
pub mod expand;
pub mod region;
pub mod shelves;

pub use bites_counter::BitesCounterUsage;
pub use bites_kitchen::BitesKitchenUsage;
pub use bites_seating::BitesSeatingUsage;
pub use expand::{expand_usage, expand_usages};
pub use region::{along_is_x, floor_height_aabb, slice_along, COUNTER_SLOT_HEIGHT};
