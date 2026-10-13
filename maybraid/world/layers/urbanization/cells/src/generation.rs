//! [`GenerationScheme`] for [`SelectedUrbanization`] and its producer channel.

use bevy::math::bounding::Aabb3d;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};

use crate::storage::UrbanizationSelection;
use crate::{select_cell, select_cell_as, SelectedUrbanization, UrbanizationExtent};

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
	fn on(extent: UrbanizationExtent, selection: &UrbanizationSelection) -> Self {
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
