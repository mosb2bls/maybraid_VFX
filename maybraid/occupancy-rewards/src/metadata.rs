//! Change-driven cached cell metadata with a Gimme spatial index.
//!
//! Publishers (world HCSG collectors or tests) write facts here. Rules read
//! only this store — never vegetation meshes or a global plant scan.

use std::collections::{BTreeMap, BTreeSet};

use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::{Resource, Vec3};
use gimme_core::{BaseScale, HashMapStore, SpatialId, TypedIndex};

use crate::identity::{
	ForestCellId, GroveLayerId, GroveTileId, OccupancyIdentity, PlantingCellId, SemanticCellKey,
	UrbanCellId, VegetationLayer,
};
use crate::tag::{AffiliationTag, ConstructionTag, GroveKindTag, LayeringTag};

const INDEX_SCALE: DVec3 = DVec3::new(100.0, 4.0, 100.0);
const COLUMN_MIN_Y: f32 = 0.0;
const COLUMN_MAX_Y: f32 = 4.0;

/// Forest-cell facts used by [`crate::ForestMatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ForestInfo {
	pub layering: Option<LayeringTag>,
}

/// One layer on a grove tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GroveInfo {
	pub layer: VegetationLayer,
	pub kind: Option<GroveKindTag>,
}

/// Published facts for one semantic cell.
#[derive(Clone, Debug, PartialEq)]
pub struct CellMetadata {
	pub key: SemanticCellKey,
	pub affiliation: Option<AffiliationTag>,
	pub forest: Option<ForestInfo>,
	pub grove: Option<GroveInfo>,
	pub constructions: Vec<ConstructionTag>,
	pub planting_extent_xz: Option<f32>,
	pub bounds: Option<Aabb3d>,
}

impl CellMetadata {
	pub fn new(key: SemanticCellKey) -> Self {
		Self {
			key,
			affiliation: None,
			forest: None,
			grove: None,
			constructions: Vec::new(),
			planting_extent_xz: None,
			bounds: None,
		}
	}

	pub fn with_affiliation(mut self, affiliation: AffiliationTag) -> Self {
		self.affiliation = Some(affiliation);
		self
	}

	pub fn with_forest(mut self, forest: ForestInfo) -> Self {
		self.forest = Some(forest);
		self
	}

	pub fn with_grove(mut self, grove: GroveInfo) -> Self {
		self.grove = Some(grove);
		self
	}

	pub fn with_construction(mut self, tag: ConstructionTag) -> Self {
		if !self.constructions.contains(&tag) {
			self.constructions.push(tag);
		}
		self
	}

	pub fn with_constructions(mut self, tags: impl IntoIterator<Item = ConstructionTag>) -> Self {
		for tag in tags {
			self = self.with_construction(tag);
		}
		self
	}

	pub fn with_planting_extent_xz(mut self, extent: f32) -> Self {
		self.planting_extent_xz = Some(extent.max(0.1));
		self
	}

	pub fn with_bounds(mut self, bounds: Aabb3d) -> Self {
		self.bounds = Some(bounds);
		self
	}

	pub fn aabb(&self) -> Aabb3d {
		self.bounds.unwrap_or_else(|| self.key.default_bounds())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishOutcome {
	Inserted,
	Unchanged,
	Replaced,
}

#[derive(Clone, Copy, Debug)]
struct IndexedCell {
	key: SemanticCellKey,
}

impl SpatialId for IndexedCell {
	type Id = SemanticCellKey;

	fn spatial_id(&self) -> Self::Id {
		self.key
	}
}

/// Aggregated facts at the actor's current occupancy.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OccupancyView {
	pub affiliation: Option<AffiliationTag>,
	pub forest: Option<ForestInfo>,
	pub layers: [Option<GroveInfo>; 4],
	pub constructions_forest: Vec<ConstructionTag>,
	pub constructions_grove: Vec<ConstructionTag>,
	pub constructions_planting: Vec<ConstructionTag>,
	pub forest_loaded: bool,
	pub grove_loaded: bool,
	pub planting_loaded: bool,
}

impl OccupancyView {
	pub fn layer(&self, layer: VegetationLayer) -> Option<GroveInfo> {
		self.layers[layer.index()]
	}

	pub fn grove_kinds(&self) -> impl Iterator<Item = GroveKindTag> + '_ {
		self.layers.iter().filter_map(|layer| layer.and_then(|info| info.kind))
	}

	pub fn constructions_at(&self, scope: crate::rule::ConstructionScope) -> &[ConstructionTag] {
		match scope {
			crate::rule::ConstructionScope::ForestCell => &self.constructions_forest,
			crate::rule::ConstructionScope::GroveTile => &self.constructions_grove,
			crate::rule::ConstructionScope::PlantingCell => &self.constructions_planting,
		}
	}

	pub fn scope_loaded(&self, scope: crate::rule::ConstructionScope) -> bool {
		match scope {
			crate::rule::ConstructionScope::ForestCell => self.forest_loaded,
			crate::rule::ConstructionScope::GroveTile => self.grove_loaded,
			crate::rule::ConstructionScope::PlantingCell => self.planting_loaded,
		}
	}
}

/// Spatial, revision-tracked cache of semantic cell facts.
#[derive(Resource)]
pub struct CellMetadataStore {
	by_key: BTreeMap<SemanticCellKey, CellMetadata>,
	by_forest: BTreeMap<ForestCellId, BTreeSet<SemanticCellKey>>,
	by_grove: BTreeMap<GroveTileId, BTreeSet<SemanticCellKey>>,
	index: Option<TypedIndex<IndexedCell, HashMapStore<IndexedCell>>>,
	revision: u64,
}

impl Default for CellMetadataStore {
	fn default() -> Self {
		Self {
			by_key: BTreeMap::new(),
			by_forest: BTreeMap::new(),
			by_grove: BTreeMap::new(),
			index: TypedIndex::new(INDEX_SCALE, HashMapStore::new()).ok(),
			revision: 0,
		}
	}
}

impl CellMetadataStore {
	pub fn revision(&self) -> u64 {
		self.revision
	}

	pub fn len(&self) -> usize {
		self.by_key.len()
	}

	pub fn is_empty(&self) -> bool {
		self.by_key.is_empty()
	}

	pub fn get(&self, key: SemanticCellKey) -> Option<&CellMetadata> {
		self.by_key.get(&key)
	}

	pub fn publish(&mut self, meta: CellMetadata) -> PublishOutcome {
		let key = meta.key;
		let bounds = meta.aabb();
		if let Some(existing) = self.by_key.get(&key) {
			if existing == &meta {
				return PublishOutcome::Unchanged;
			}
			self.unlink(key);
			self.by_key.insert(key, meta);
			self.link(key, bounds);
			self.revision = self.revision.saturating_add(1);
			return PublishOutcome::Replaced;
		}
		self.by_key.insert(key, meta);
		self.link(key, bounds);
		self.revision = self.revision.saturating_add(1);
		PublishOutcome::Inserted
	}

	pub fn evict(&mut self, key: SemanticCellKey) -> Option<CellMetadata> {
		let removed = self.by_key.remove(&key)?;
		self.unlink(key);
		self.revision = self.revision.saturating_add(1);
		Some(removed)
	}

	/// Planting cell that owns `position` if a grove publisher supplied an extent.
	pub fn planting_at(&self, tile: GroveTileId, position: Vec3) -> Option<PlantingCellId> {
		let extent = self.planting_extent_for(tile)?;
		Some(PlantingCellId::containing(tile, VegetationLayer::UpperCanopy, extent, position))
	}

	pub fn planting_extent_for(&self, tile: GroveTileId) -> Option<f32> {
		for layer in VegetationLayer::ALL {
			if let Some(extent) = self
				.get(SemanticCellKey::GroveLayer(GroveLayerId { tile, layer }))
				.and_then(|meta| meta.planting_extent_xz)
			{
				return Some(extent);
			}
		}
		None
	}

	pub fn urban_at(&self, position: Vec3) -> Option<UrbanCellId> {
		if !position.is_finite() {
			return None;
		}
		let mut found = None;
		self.for_overlapping(point_region(position), |meta| {
			if let SemanticCellKey::Urban(id) = meta.key {
				if aabb_contains_xz(meta.aabb(), position) {
					found = Some(id);
				}
			}
		});
		found
	}

	pub fn view(&self, identity: &OccupancyIdentity) -> OccupancyView {
		let mut view = OccupancyView::default();
		if let Some(keys) = self.by_forest.get(&identity.forest) {
			view.forest_loaded = true;
			for key in keys {
				let Some(meta) = self.by_key.get(key) else {
					continue;
				};
				if view.affiliation.is_none() {
					view.affiliation = meta.affiliation;
				}
				if view.forest.is_none() {
					view.forest = meta.forest;
				}
				push_unique(&mut view.constructions_forest, &meta.constructions);
			}
		}
		if let Some(keys) = self.by_grove.get(&identity.grove) {
			view.grove_loaded = true;
			for key in keys {
				let Some(meta) = self.by_key.get(key) else {
					continue;
				};
				if view.affiliation.is_none() {
					view.affiliation = meta.affiliation;
				}
				if view.forest.is_none() {
					view.forest = meta.forest;
				}
				if let SemanticCellKey::GroveLayer(id) = meta.key {
					view.layers[id.layer.index()] =
						meta.grove.or(Some(GroveInfo { layer: id.layer, kind: None }));
				}
				push_unique(&mut view.constructions_grove, &meta.constructions);
			}
		}
		if let Some(planting) = identity.planting {
			if let Some(cell) = self.get(SemanticCellKey::Planting(planting)) {
				view.planting_loaded = true;
				push_unique(&mut view.constructions_planting, &cell.constructions);
			}
		}
		view
	}

	fn for_overlapping(&self, region: Aabb3d, mut visit: impl FnMut(&CellMetadata)) {
		let region = column_aabb(region);
		if let Some(index) = &self.index {
			let levels = query_levels(index, region);
			let mut seen = Vec::new();
			for (cell, _) in index.query_values(region, levels) {
				if seen.contains(&cell.key) {
					continue;
				}
				seen.push(cell.key);
				if let Some(meta) = self.by_key.get(&cell.key) {
					visit(meta);
				}
			}
			return;
		}
		for meta in self.by_key.values() {
			if aabbs_overlap_xz(meta.aabb(), region) {
				visit(meta);
			}
		}
	}

	fn link(&mut self, key: SemanticCellKey, bounds: Aabb3d) {
		if let Some(forest) = key.forest() {
			self.by_forest.entry(forest).or_default().insert(key);
		}
		if let Some(grove) = key.grove() {
			self.by_grove.entry(grove).or_default().insert(key);
		}
		if let Some(index) = self.index.as_mut() {
			let _ = index.insert_value(IndexedCell { key }, column_aabb(bounds));
		}
	}

	fn unlink(&mut self, key: SemanticCellKey) {
		if let Some(forest) = key.forest() {
			if let Some(keys) = self.by_forest.get_mut(&forest) {
				keys.remove(&key);
				if keys.is_empty() {
					self.by_forest.remove(&forest);
				}
			}
		}
		if let Some(grove) = key.grove() {
			if let Some(keys) = self.by_grove.get_mut(&grove) {
				keys.remove(&key);
				if keys.is_empty() {
					self.by_grove.remove(&grove);
				}
			}
		}
		if let Some(index) = self.index.as_mut() {
			index.remove_value(key);
		}
	}
}

fn push_unique(dst: &mut Vec<ConstructionTag>, src: &[ConstructionTag]) {
	for tag in src {
		if !dst.contains(tag) {
			dst.push(*tag);
		}
	}
}

fn column_aabb(bounds: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(bounds.min.x, COLUMN_MIN_Y, bounds.min.z),
		Vec3::new(bounds.max.x, COLUMN_MAX_Y, bounds.max.z),
	)
}

fn point_region(position: Vec3) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(position.x - 0.01, 0.0, position.z - 0.01),
		Vec3::new(position.x + 0.01, 1.0, position.z + 0.01),
	)
}

fn aabb_contains_xz(bounds: Aabb3d, position: Vec3) -> bool {
	position.x >= bounds.min.x
		&& position.x < bounds.max.x
		&& position.z >= bounds.min.z
		&& position.z < bounds.max.z
}

fn aabbs_overlap_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

fn query_levels(
	index: &TypedIndex<IndexedCell, HashMapStore<IndexedCell>>,
	region: Aabb3d,
) -> std::ops::RangeInclusive<u32> {
	let query_level = BaseScale::new(index.grid().base_scale())
		.map(|base| base.insertion_level(&region))
		.unwrap_or(0);
	let max = query_level.max(index.grid().max_level());
	0..=max
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tag::ConstructionTag;

	fn grove_meta(tile: GroveTileId, layer: VegetationLayer, tag: ConstructionTag) -> CellMetadata {
		CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId { tile, layer }))
			.with_grove(GroveInfo { layer, kind: Some(GroveKindTag::named("TradeWinds")) })
			.with_construction(tag)
	}

	#[test]
	fn republish_equal_payload_does_not_bump_revision() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		let tile = GroveTileId::containing(Vec3::ZERO);
		let meta = grove_meta(tile, VegetationLayer::UpperCanopy, ConstructionTag::SOPES_BANYAN);
		assert_eq!(store.publish(meta.clone()), PublishOutcome::Inserted);
		let revision = store.revision();
		assert_eq!(store.publish(meta), PublishOutcome::Unchanged);
		assert_eq!(store.revision(), revision);
		Ok(())
	}

	#[test]
	fn view_unions_overlapping_constructions_without_duplicating_tags() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		let identity = OccupancyIdentity::from_position(Vec3::ZERO);
		store.publish(grove_meta(
			identity.grove,
			VegetationLayer::UpperCanopy,
			ConstructionTag::SOPES_BANYAN,
		));
		store.publish(grove_meta(
			identity.grove,
			VegetationLayer::LowerCanopy,
			ConstructionTag::SOPES_BANYAN,
		));
		store.publish(grove_meta(
			identity.grove,
			VegetationLayer::Understory,
			ConstructionTag::named("other"),
		));
		let view = store.view(&identity);
		assert!(view.grove_loaded);
		assert_eq!(view.constructions_grove.len(), 2);
		assert!(view.constructions_grove.contains(&ConstructionTag::SOPES_BANYAN));
		Ok(())
	}

	#[test]
	fn absent_metadata_is_unloaded() -> anyhow::Result<()> {
		let store = CellMetadataStore::default();
		let view = store.view(&OccupancyIdentity::from_position(Vec3::ZERO));
		assert!(!view.forest_loaded);
		assert!(!view.grove_loaded);
		assert!(view.constructions_grove.is_empty());
		Ok(())
	}

	#[test]
	fn planting_extent_is_read_from_grove_metadata() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		let identity = OccupancyIdentity::from_position(Vec3::ZERO);
		store.publish(
			CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId {
				tile: identity.grove,
				layer: VegetationLayer::UpperCanopy,
			}))
			.with_planting_extent_xz(25.0),
		);
		let planting = store
			.planting_at(identity.grove, Vec3::ZERO)
			.ok_or_else(|| anyhow::anyhow!("expected planting cell"))?;
		assert_eq!(planting.tile, identity.grove);
		assert_eq!(planting.cx, 0);
		Ok(())
	}
}
