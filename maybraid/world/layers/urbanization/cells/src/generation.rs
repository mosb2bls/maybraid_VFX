//! [`GenerationScheme`] for [`SelectedUrbanization`] and its producer channel.

use bevy::math::bounding::Aabb3d;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};

use crate::storage::UrbanizationSelection;
use crate::{
	select_cell, select_cell_as, DevelopmentLeaf, SelectedUrbanization, UrbanDevelopmentKind,
	UrbanizationExtent,
};

fn overlaps_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
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
	fn on(extent: UrbanizationExtent, selection: &UrbanizationSelection) -> Self {
		match selection.kind {
			Some(kind) => select_cell_as(extent, selection.noise, kind),
			None => select_cell(extent, selection.noise),
		}
	}

	/// Non-empty guillotine leaves whose bounds overlap `region` on XZ.
	pub fn filled_leaves_overlapping(
		&self,
		region: Aabb3d,
	) -> impl Iterator<Item = &DevelopmentLeaf> {
		self.leaves.iter().filter(move |leaf| {
			leaf.kind != UrbanDevelopmentKind::Empty && overlaps_xz(region, leaf.bounds)
		})
	}

	/// Development leaf ids originating in `region`.
	///
	/// Materializes overlapping [`SelectedUrbanization`] cells through `cx` so
	/// later consumers reuse the same cached selections.
	pub fn original_leaf_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids: Vec<OriginalId> = cx
			.get_or_generate_in::<Self>(region)
			.iter()
			.flat_map(|selected| selected.filled_leaves_overlapping(region))
			.map(|leaf| OriginalId(leaf.id()))
			.collect();
		ids.sort_unstable();
		ids.dedup();
		ids
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::hcsg::GenerationContext;
	use lod::hcsg::{universal_bounds, HcsgStorage};
	use procedural_common::NoiseParams;
	use std::sync::Arc;

	fn frontier_selection() -> UrbanizationSelection {
		UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(crate::UrbanizationKind::Frontier),
		}
	}

	fn seeded(selection: UrbanizationSelection) -> HcsgStorage {
		let storage = HcsgStorage::default();
		storage.seed(selection, universal_bounds());
		storage
	}

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
	fn leaf_ids_match_cached_selection_leaves() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 2);
		let storage = seeded(frontier_selection());
		let mut cx = GenerationContext::new(&storage);
		let leaves = SelectedUrbanization::original_leaf_ids_for(&mut cx, region);
		anyhow::ensure!(!leaves.is_empty(), "frontier cells produce filled leaves");

		let mut from_cells = Vec::new();
		for selected in cx.get_or_generate_in::<SelectedUrbanization>(region) {
			from_cells.extend(
				selected.filled_leaves_overlapping(region).map(|leaf| OriginalId(leaf.id())),
			);
		}
		from_cells.sort_unstable();
		from_cells.dedup();
		anyhow::ensure!(leaves == from_cells);

		for extent in UrbanizationExtent::cells_overlapping(region) {
			anyhow::ensure!(
				cx.get::<SelectedUrbanization>(extent.id()).is_some(),
				"leaf discovery stores the owning selection"
			);
		}
		Ok(())
	}

	#[test]
	fn leaf_query_includes_only_overlapping_filled_leaves() -> Result<()> {
		let storage = seeded(frontier_selection());
		let mut cx = GenerationContext::new(&storage);
		let cell = UrbanizationExtent::from_cell_index(1, -1);
		let selected = cx
			.get_or_generate::<SelectedUrbanization>(cell.id())
			.ok_or_else(|| anyhow::anyhow!("selection"))?;
		let Some(leaf) = selected
			.leaves
			.iter()
			.find(|leaf| leaf.kind != UrbanDevelopmentKind::Empty)
			.cloned()
		else {
			anyhow::bail!("expected a filled leaf");
		};

		let tight = leaf.bounds;
		let ids = SelectedUrbanization::original_leaf_ids_for(&mut cx, tight);
		anyhow::ensure!(ids.contains(&OriginalId(leaf.id())), "owning leaf is discovered");
		for OriginalId(id) in &ids {
			let owner = UrbanizationExtent::owning_leaf(*id)
				.ok_or_else(|| anyhow::anyhow!("leaf owner"))?;
			let stored = cx
				.get::<SelectedUrbanization>(owner.id())
				.ok_or_else(|| anyhow::anyhow!("cached owner"))?;
			let found =
				stored.leaf(*id).ok_or_else(|| anyhow::anyhow!("leaf in cached selection"))?;
			anyhow::ensure!(found.kind != UrbanDevelopmentKind::Empty);
			anyhow::ensure!(overlaps_xz(tight, found.bounds));
		}

		for other in selected.filled_leaves_overlapping(tight) {
			anyhow::ensure!(ids.contains(&OriginalId(other.id())));
		}
		Ok(())
	}

	#[test]
	fn overlapping_leaf_queries_reuse_cached_selections() -> Result<()> {
		let storage = seeded(frontier_selection());
		let first = UrbanizationExtent::xz_radius_aabb(bevy::math::Vec3::ZERO, 1800.0);
		let second =
			UrbanizationExtent::xz_radius_aabb(bevy::math::Vec3::new(1200.0, 0.0, 0.0), 1800.0);
		let mut cx = GenerationContext::new(&storage);
		let _ = SelectedUrbanization::original_leaf_ids_for(&mut cx, first);

		let mut cached = Vec::new();
		for extent in UrbanizationExtent::cells_overlapping(first) {
			let value = cx
				.get::<SelectedUrbanization>(extent.id())
				.ok_or_else(|| anyhow::anyhow!("first query stores selections"))?;
			cached.push((extent.id(), value));
		}

		let _ = SelectedUrbanization::original_leaf_ids_for(&mut cx, second);
		for (id, first_value) in cached {
			let again = cx
				.get_or_generate::<SelectedUrbanization>(id)
				.ok_or_else(|| anyhow::anyhow!("overlap still present"))?;
			anyhow::ensure!(
				Arc::ptr_eq(&first_value, &again),
				"overlapping query reuses the published selection"
			);
		}
		Ok(())
	}

	#[test]
	fn selection_builds_from_seeded_root() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 2);
		let storage = seeded(frontier_selection());
		let mut cx = GenerationContext::new(&storage);
		for OriginalId(id) in cx.original_ids_for::<SelectedUrbanization>(region) {
			anyhow::ensure!(cx.get_or_generate::<SelectedUrbanization>(id).is_some());
		}
		Ok(())
	}
}
