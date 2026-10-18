//! Watershed correction cells ([`WATERSHED_CORRECTION.md`] bones).
//!
//! Pipeline:
//! ```text
//! PocketWaters{High,Low}Pass (authored enum → HydrologyNodes)
//!   → HydroComplexCell (origin grid: union nodes from both passes)
//!   → CarvingCell / RimmingCell / AproningCell (stage bones)
//!   → Terrain applies HydroComplex (internal carve → rim → apron)
//! ```

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::watersheds::config::WatershedConfigs;
use crate::terrain::watersheds::high_pass::PocketWatersHighPass;
use crate::terrain::watersheds::leaf_kind::WatershedLeafBounds;
use crate::terrain::watersheds::low_pass::PocketWatersLowPass;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use procedural_common::Bounds2;
use std::sync::Arc;
use terrain_watersheds::{CorrectionStage, HydroComplex, HydroNode};

/// Origin-grid hydrology complex: unions hydrology nodes from both pocket-water passes.
#[derive(Debug, Clone, Component)]
pub struct HydroComplexCell {
	pub cell: Aabb3d,
	pub complex: Arc<HydroComplex>,
	/// Pocket-water leaf metadata discovered while building this complex (both bands).
	pub marazion_leaves: Vec<WatershedLeafBounds>,
}

impl HydroComplexCell {
	/// Indexed complex when it has hydrology members.
	pub fn indexed(&self) -> Option<&Arc<HydroComplex>> {
		(!self.complex.is_empty()).then_some(&self.complex)
	}
}

fn aabb_to_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn cell_seed(cell: Aabb3d, salt: u32) -> u32 {
	salt.wrapping_add(cell.min.x.to_bits().wrapping_mul(73856093))
		.wrapping_add(cell.min.z.to_bits().wrapping_mul(19349663))
}

/// Origin-grid root for watershed correction; both pocket-water passes are
/// pulled by region, so their pocket / pre-pocket stacks stay out of these bounds.

impl hcsg::GenerationScheme for HydroComplexCell {
	lod::hcsg_index_scale!(crate::terrain::index::DURHAM_INDEX_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		TerrainCellLayout::origin_ids_in(cx, region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let seed = cx.get_or_generate::<WatershedConfigs>(Id::Universal)?.seed;
		let high = cx.get_or_generate_all_in::<PocketWatersHighPass>(cell)?;
		let low = cx.get_or_generate_all_in::<PocketWatersLowPass>(cell)?;
		let marazion_leaves = high
			.iter()
			.map(|leaf| leaf.leaf_bounds())
			.chain(low.iter().map(|leaf| leaf.leaf_bounds()))
			.collect();
		let nodes = high
			.iter()
			.flat_map(|pass| pass.hydro_nodes())
			.chain(low.iter().flat_map(|pass| pass.hydro_nodes()));
		Some((Self::union(cell, seed, nodes, marazion_leaves), cell))
	}
}

impl HydroComplexCell {
	/// The complex of `nodes` whose correction reaches `cell`.
	pub fn union(
		cell: Aabb3d,
		seed: u32,
		nodes: impl IntoIterator<Item = HydroNode>,
		marazion_leaves: Vec<WatershedLeafBounds>,
	) -> Self {
		let cell_bounds = aabb_to_bounds2(cell);
		let hydrology = nodes
			.into_iter()
			.filter(|node| node.correction_intersects(cell_bounds))
			.collect();
		let complex =
			Arc::new(HydroComplex::new(cell_bounds, cell_seed(cell, seed)).with_hydro(hydrology));
		Self { cell, complex, marazion_leaves }
	}
}

/// Origin-cell carve stage over the cellular [`HydroComplexCell`].
#[derive(Debug, Clone, Component)]
pub struct WatershedCarvingCell {
	pub cell: Aabb3d,
	pub complex: Option<Arc<HydroComplex>>,
}

/// Rim correction (raise-only bank toward shelf_anchor + rim_lift).
#[derive(Debug, Clone, Component)]
pub struct WatershedRimmingCell {
	pub cell: Aabb3d,
	pub complex: Option<Arc<HydroComplex>>,
}

/// Apron correction (fade from bank toward identity).
#[derive(Debug, Clone, Component)]
pub struct WatershedAproningCell {
	pub cell: Aabb3d,
	pub complex: Option<Arc<HydroComplex>>,
}

/// Stage cells are views over [`HydroComplexCell`]: same origin ids, and that
/// is their only dependency.
macro_rules! impl_correction_stage_cell {
	($Cell:ty) => {
		impl hcsg::GenerationScheme for $Cell {
			lod::hcsg_index_scale!($crate::terrain::index::DURHAM_INDEX_SCALE);
			fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
				cx.original_ids_for::<HydroComplexCell>(region)
			}

			fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
				let complex_cell = cx.get_or_generate::<HydroComplexCell>(id)?;
				let cell = complex_cell.cell;
				Some((Self { cell, complex: complex_cell.indexed().cloned() }, cell))
			}
		}
	};
}

impl_correction_stage_cell!(WatershedCarvingCell);
impl_correction_stage_cell!(WatershedRimmingCell);
impl_correction_stage_cell!(WatershedAproningCell);

impl WatershedCarvingCell {
	pub const STAGE: CorrectionStage = CorrectionStage::Carve;
}

impl WatershedRimmingCell {
	pub const STAGE: CorrectionStage = CorrectionStage::Rim;
}

impl WatershedAproningCell {
	pub const STAGE: CorrectionStage = CorrectionStage::Apron;
}
