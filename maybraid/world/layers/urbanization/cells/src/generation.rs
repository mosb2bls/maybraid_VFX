//! [`GenerationScheme`] for [`SelectedUrbanization`] and its producer channel.

use bevy::math::bounding::Aabb3d;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};

use crate::storage::UrbanizationSelection;
use crate::{
	select_cell, select_cell_as, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
};

fn overlaps_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

/// Non-empty guillotine leaf ids overlapping `region`, derived from session
/// `selection` without materializing [`SelectedUrbanization`] in storage.
pub fn urbanization_leaf_original_ids(
	region: Aabb3d,
	selection: &UrbanizationSelection,
) -> Vec<OriginalId> {
	UrbanizationExtent::cells_overlapping(region)
		.into_iter()
		.flat_map(|extent| {
			let selected = SelectedUrbanization::on(extent, selection);
			selected.leaves.into_iter().filter(move |leaf| {
				leaf.kind != UrbanDevelopmentKind::Empty && overlaps_xz(region, leaf.bounds)
			})
		})
		.map(|leaf| OriginalId(leaf.id()))
		.collect()
}

/// Urbanization selection generate ring around the camera (metres).
pub const DEVELOPMENT_GENERATE_RADIUS_M: f32 = 3000.0;

/// Urbanization present ring around the camera (metres).
pub const DEVELOPMENT_PRESENT_RADIUS_M: f32 = 1000.0;

/// Producer channel for urbanization selection: the generate ring around the viewer.
pub struct UrbanizationWindow;

impl hcsg::GenerationScheme for SelectedUrbanization {
	lod::hcsg_index_scale!(crate::storage::SELECTION_INDEX_SCALE);

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		UrbanizationExtent::original_ids_overlapping(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = UrbanizationExtent::from_id(id)?;
		let selection = cx.get_or_generate::<UrbanizationSelection>(Id::Universal)?;
		Some((Self::on(extent, &selection), extent.aabb()))
	}
}

impl SelectedUrbanization {
	/// `extent`'s cell as the session's `selection` picks it.
	pub fn on(extent: UrbanizationExtent, selection: &UrbanizationSelection) -> Self {
		match selection.kind {
			Some(kind) => select_cell_as(extent, selection.noise, kind),
			None => select_cell(extent, selection.noise),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::hcsg::GenerationContext;
	use lod::hcsg::{universal_bounds, HcsgStorage};
	use procedural_common::NoiseParams;

	#[test]
	fn urbanization_original_ids_are_overlapping_cells() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 1);
		let storage = HcsgStorage::default();
		let mut cx = GenerationContext::new(&storage);
		let ids = cx.original_ids_for::<SelectedUrbanization>(region);
		assert_eq!(ids.len(), 9);
		Ok(())
	}

	#[test]
	fn leaf_original_ids_match_materialized_selection() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 2);
		let selection = UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(crate::UrbanizationKind::Frontier),
		};
		let storage = HcsgStorage::default();
		storage.seed(selection, universal_bounds());
		let mut cx = GenerationContext::new(&storage);
		let session = cx.get::<UrbanizationSelection>(Id::Universal).expect("seeded root");
		let pure = urbanization_leaf_original_ids(region, session.as_ref());
		let mut materialized = Vec::new();
		for extent in UrbanizationExtent::cells_overlapping(region) {
			let selected = cx.get_or_generate::<SelectedUrbanization>(extent.id()).expect("cell");
			materialized.extend(
				selected
					.leaves
					.iter()
					.filter(|leaf| leaf.kind != UrbanDevelopmentKind::Empty)
					.map(|leaf| OriginalId(leaf.id())),
			);
		}
		materialized.sort_unstable();
		materialized.dedup();
		let mut pure_sorted = pure;
		pure_sorted.sort_unstable();
		pure_sorted.dedup();
		anyhow::ensure!(pure_sorted == materialized);
		Ok(())
	}

	#[test]
	fn selection_builds_from_seeded_root() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 2);
		let selection = UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(crate::UrbanizationKind::Frontier),
		};
		let storage = HcsgStorage::default();
		storage.seed(selection, universal_bounds());
		let mut cx = GenerationContext::new(&storage);
		for OriginalId(id) in cx.original_ids_for::<SelectedUrbanization>(region) {
			anyhow::ensure!(cx.get_or_generate::<SelectedUrbanization>(id).is_some());
		}
		Ok(())
	}
}
