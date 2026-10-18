//! [`HcsgStorage`] records for urbanization selection.

use std::sync::Arc;

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::DVec3;
use bevy::prelude::*;
use lod::gen::Id;
use lod::hcsg::{Busy, HcsgStorage, StoredEntry};
use procedural_common::NoiseParams;

use crate::{
	DevelopmentLeaf, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationKind, DEFAULT_URBANIZATION_EXTENT_XZ,
};

/// Root input every [`SelectedUrbanization`] is thrown from.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct UrbanizationSelection {
	pub noise: NoiseParams,
	/// When set, every cell uses this kind instead of Hopscotch (playground pin).
	pub kind: Option<UrbanizationKind>,
}

lod::seeded_root!(UrbanizationSelection);

/// Selections are bucketed one extent (1600 m) apart.
pub(crate) const SELECTION_INDEX_SCALE: DVec3 =
	DVec3::new(DEFAULT_URBANIZATION_EXTENT_XZ as f64, 1.0, DEFAULT_URBANIZATION_EXTENT_XZ as f64);
/// Urbanization reads over [`HcsgStorage`]. GET only: nothing is selected here.
pub trait UrbanizationStorage {
	fn selected(&self, id: Id) -> Result<Option<Arc<SelectedUrbanization>>, Busy>;

	/// Stored selections whose extents overlap `region`.
	fn selected_overlapping(
		&self,
		region: Aabb3d,
	) -> Result<Vec<(Id, StoredEntry<Arc<SelectedUrbanization>>)>, Busy>;

	/// Advances whenever a selection is stored or dropped.
	fn urbanization_revision(&self) -> Result<u64, Busy>;

	/// The stored leaf whose [`DevelopmentLeaf::id`] is `id`.
	fn leaf(&self, id: Id) -> Result<Option<DevelopmentLeaf>, Busy>;

	/// Stored non-empty leaves whose bounds intersect `region`.
	fn filled_leaves_overlapping(&self, region: Aabb3d) -> Result<Vec<DevelopmentLeaf>, Busy>;
}

impl UrbanizationStorage for HcsgStorage {
	fn selected(&self, id: Id) -> Result<Option<Arc<SelectedUrbanization>>, Busy> {
		self.try_entry::<SelectedUrbanization>(id)
			.map(|entry| entry.map(|stored| stored.value))
	}

	fn selected_overlapping(
		&self,
		region: Aabb3d,
	) -> Result<Vec<(Id, StoredEntry<Arc<SelectedUrbanization>>)>, Busy> {
		let ids = self.try_overlapping::<SelectedUrbanization>(region)?;
		let mut out = Vec::with_capacity(ids.len());
		for id in ids {
			if let Some(entry) = self.try_entry::<SelectedUrbanization>(id)? {
				out.push((id, entry));
			}
		}
		Ok(out)
	}

	fn urbanization_revision(&self) -> Result<u64, Busy> {
		self.try_membership_revision::<SelectedUrbanization>()
	}

	fn leaf(&self, id: Id) -> Result<Option<DevelopmentLeaf>, Busy> {
		let Some(extent) = UrbanizationExtent::owning_leaf(id) else {
			return Ok(None);
		};
		Ok(self
			.selected(extent.id())?
			.and_then(|selected| selected.as_ref().leaf(id).cloned()))
	}

	fn filled_leaves_overlapping(&self, region: Aabb3d) -> Result<Vec<DevelopmentLeaf>, Busy> {
		Ok(self
			.selected_overlapping(region)?
			.into_iter()
			.flat_map(|(_, entry)| entry.value.leaves.clone())
			.filter(|leaf| {
				leaf.kind != UrbanDevelopmentKind::Empty && region.intersects(&leaf.bounds)
			})
			.collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::hcsg::universal_bounds;
	use lod::hcsg::GenerationContext;

	fn seeded(selection: UrbanizationSelection) -> HcsgStorage {
		let storage = HcsgStorage::default();
		storage.seed(selection, universal_bounds());
		storage
	}

	fn storage_busy(busy: Busy) -> anyhow::Error {
		anyhow::anyhow!("HcsgStorage busy: {busy:?}")
	}

	#[test]
	fn selection_is_idempotent() -> Result<()> {
		let storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1.0, 0.01, 1.0, 1),
			kind: None,
		});
		let id = UrbanizationExtent::default_cell().id();
		let mut cx = GenerationContext::new(&storage);
		cx.get_or_generate::<SelectedUrbanization>(id);
		let first = storage
			.try_entry::<SelectedUrbanization>(id)
			.map_err(storage_busy)?
			.map(|entry| entry.version);
		cx.get_or_generate::<SelectedUrbanization>(id);
		anyhow::ensure!(first.is_some(), "selection is stored");
		anyhow::ensure!(
			storage
				.try_entry::<SelectedUrbanization>(id)
				.map_err(storage_busy)?
				.map(|entry| entry.version)
				== first,
			"a second request reuses the stored selection"
		);
		Ok(())
	}

	#[test]
	fn selection_reads_the_seeded_root() -> Result<()> {
		let storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(UrbanizationKind::Frontier),
		});
		let extent = UrbanizationExtent::default_cell();
		let selected = GenerationContext::new(&storage)
			.get_or_generate::<SelectedUrbanization>(extent.id())
			.ok_or_else(|| anyhow::anyhow!("selection"))?;
		anyhow::ensure!(selected.kind == UrbanizationKind::Frontier, "the pin is honored");
		anyhow::ensure!(selected.extent == extent);

		let unseeded = HcsgStorage::default();
		anyhow::ensure!(
			GenerationContext::new(&unseeded)
				.get_or_generate::<SelectedUrbanization>(extent.id())
				.is_none(),
			"nothing is selected before the root is seeded"
		);
		Ok(())
	}

	#[test]
	fn leaf_lookup_finds_a_stored_leaf() -> Result<()> {
		let storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(UrbanizationKind::Frontier),
		});
		let extent = UrbanizationExtent::from_cell_index(1, -1);
		let leaf = GenerationContext::new(&storage)
			.get_or_generate::<SelectedUrbanization>(extent.id())
			.and_then(|selected| selected.leaves.first().cloned())
			.ok_or_else(|| anyhow::anyhow!("expected leaves"))?;
		anyhow::ensure!(storage.leaf(leaf.id()).map_err(storage_busy)? == Some(leaf));
		Ok(())
	}
}
