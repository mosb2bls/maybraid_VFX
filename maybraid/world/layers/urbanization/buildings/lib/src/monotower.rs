//! A monotower is a tower of a single type of storey floor plan.
//!
//! Because the mappings of shafts onto the floor plans to the storeys are one-to-one,
//! we can ensure good continuity.
//!
//! The monotower maps inbound shafts onto aligned storey slots. Stairwells and
//! roof are owned by the tower consumer (a Richmond development).
//!
//! When monotowers fit an [`Aabb3d`](bevy_math::bounding::Aabb3d), they choose a
//! floor height from a range and then adjust to have a whole number of storeys.
//!
//! Monotowers typically follow a Parameterized → FloorPlan → UsagePlan approach:
//! floor plans carry the shared shell (and shaft slots); usage plans paint
//! residuals per storey (commercial / livable / …).
//!
//! Concrete towers live under [`crate::monotower`] submodules (e.g.
//! [`les_halles`](crate::monotower::les_halles)).

pub mod apartment;
pub mod les_halles;

pub use apartment::{
	ApartmentMonotower, SingleHighrise, SingleHighriseFloorPlan, SingleHighrisePlan,
	SingleHighriseShaftSlot, SingleHighriseStorey,
};
pub use les_halles::{MixedUseLesHallesMonotower, MixedUseLesHallesStorey};
