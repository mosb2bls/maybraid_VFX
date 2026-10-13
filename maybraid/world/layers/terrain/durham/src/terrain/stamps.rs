//! Stamp landform stamps on per-family dual-band guillotine partitions.
//!
//! Each family owns independent **low-pass** (detail) and **high-pass**
//! (regional) controller grids — different guillotine cell-size ranges,
//! controller roots, origin offsets, cut seeds, likelihoods, spatial
//! correlation lengths, and stamp **strength** ranges. Leaf seams therefore do
//! not coincide across families or bands.
//!
//! Horizontal footprint follows leaf/cell size (`*_frac`). Vertical amplitude
//! is driven by per-leaf [`terrain_stamps::StampStrength`] sampled from
//! each band's `strength: (min, max)`.
//!
//! Stack per band:
//!
//! `ControllerLayout` → `ControllerCell` (cuts) → leaf `Id` → `StampCell`
//!
//! There is no stored guillotine-identity layer: a valid leaf [`lod::gen::Id`]
//! down-levels to its cell. [`crate::terrain::PreWatershedTerrain`] pulls
//! stamp cells through its `GenerationContext` (high-pass first,
//! then low-pass); controllers and layouts never appear in its bounds.
//!
//! Modules:
//! - [`configs`] — universal dual-band cut + stamp + likelihood params
//! - [`shared`] — offset grids, cut helpers, leaf discovery, occupancy
//! - [`plateau`] / [`massif`] / [`canyon`] / [`pocket_water`] / [`rolling`] /
//!   [`valley`] — independent family stacks

pub mod canyon;
pub mod configs;
pub mod family_macro;
pub mod massif;
pub mod plateau;
pub mod pocket_water;
pub mod rolling;
pub mod shared;
pub mod valley;

#[cfg(test)]
mod tests;

pub use canyon::{
	CanyonHighPassControllerCell, CanyonHighPassControllerLayout, CanyonHighPassStampCell,
	CanyonLowPassControllerCell, CanyonLowPassControllerLayout, CanyonLowPassStampCell,
};
pub use configs::{DualBandFamilyConfig, FamilyGuillotineConfig, TerrainStampConfigs};
pub use massif::{
	MassifHighPassControllerCell, MassifHighPassControllerLayout, MassifHighPassStampCell,
	MassifLowPassControllerCell, MassifLowPassControllerLayout, MassifLowPassStampCell,
};
pub use plateau::{
	PlateauHighPassControllerCell, PlateauHighPassControllerLayout, PlateauHighPassStampCell,
	PlateauLowPassControllerCell, PlateauLowPassControllerLayout, PlateauLowPassStampCell,
};
/// Compatibility alias: low-pass plateau layout (detail band).
pub type PlateauControllerLayout = PlateauLowPassControllerLayout;
pub use pocket_water::{
	PocketWaterHighPassControllerCell, PocketWaterHighPassControllerLayout,
	PocketWaterHighPassStampCell, PocketWaterLowPassControllerCell,
	PocketWaterLowPassControllerLayout, PocketWaterLowPassStampCell,
};
pub use rolling::{
	RollingHighPassControllerCell, RollingHighPassControllerLayout, RollingHighPassStampCell,
	RollingLowPassControllerCell, RollingLowPassControllerLayout, RollingLowPassStampCell,
};
pub use shared::StampLeaf;
pub use valley::{
	ValleyHighPassControllerCell, ValleyHighPassControllerLayout, ValleyHighPassStampCell,
	ValleyLowPassControllerCell, ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
