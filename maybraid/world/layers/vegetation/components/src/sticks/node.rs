//! Stick IR node: geometry + placement.

use bevy::light::NotShadowCaster;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Vec3, Visibility};
use bevy::scene::prelude::{bsn, Scene};
use lod::lod_ref::LodRef;
use lod::scene::{
	cull_offset_bands_from_factor, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus,
};
use lod::SceneChunk;
use material_ref::MaterialRef;
use scene_ref::{MultiSceneMerge, MultiScenePart};

use crate::assets::AssetPath;
use crate::lod_band::warm_mesh_lod_culls;
use crate::lod_host::{posed_foliage_multi_scene_merge, posed_material_asset_tier};
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children};
use crate::sticks::collection::{
	StickCollection, StickMember, STICK_COLLECTION_HIGH_METERS, STICK_COLLECTION_LOW_METERS,
	STICK_COLLECTION_MEDIUM_METERS,
};
use crate::sticks::geometry::StickGeometry;
use crate::sticks::probe::StickLodProbe;

/// Authoring IR for a stick / trunk segment — also the fine-phase [`LodScene`] host component.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct StickNode {
	pub geometry: StickGeometry,
	pub placement: Placement,
	/// Deferred material. Defaults to [`MaterialRef::default()`] (green standard);
	/// higher-order types set stick / palette as needed.
	pub material: MaterialRef,
	/// When set, this node is one merged collection (geometry is unused for emission).
	pub collection: Option<StickCollection>,
}

impl StickNode {
	pub fn new(geometry: StickGeometry, placement: Placement) -> Self {
		Self { geometry, placement, material: MaterialRef::default(), collection: None }
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = material;
		self
	}

	/// Branch / connector segment using `vegetation/sticks/standard/001_*` GLBs.
	pub fn segment(placement: Placement) -> Self {
		Self::new(StickGeometry::Segment, placement)
	}

	/// Standard stick from a directed segment (base at `start`, along `start → end`).
	///
	/// Girth uses `radius` at the segment start. Degenerate (near-zero length) edges
	/// return [`None`]. Defaults to [`StickGeometry::Segment`].
	pub fn from_segment(start: Vec3, end: Vec3, radius: f32) -> Option<Self> {
		Self::from_segment_geometry(start, end, radius, StickGeometry::Segment)
	}

	/// Like [`Self::from_segment`], with an explicit geometry (segment vs trunk kit).
	pub fn from_segment_geometry(
		start: Vec3,
		end: Vec3,
		radius: f32,
		geometry: StickGeometry,
	) -> Option<Self> {
		let ray = end - start;
		let len_sq = ray.length_squared();
		if len_sq < 1e-12 {
			return None;
		}
		let length = len_sq.sqrt();
		let placement = Placement::stick_segment(start, ray, length, radius)?;
		Some(Self::standard(geometry, placement))
	}

	/// Trunk geometry using `vegetation/sticks/standard/trunk_001_*` GLBs.
	pub fn trunk(placement: Placement) -> Self {
		Self::new(StickGeometry::Trunk, placement)
	}

	pub fn standard(geometry: StickGeometry, placement: Placement) -> Self {
		Self::new(geometry, placement)
	}

	/// Stick collection under one LOD parent (merge thinning by collection extent).
	///
	/// Parent [`Placement`] is usually identity when members are already posed.
	pub fn collection(collection: StickCollection, placement: Placement) -> Self {
		Self {
			geometry: StickGeometry::Segment,
			placement,
			material: MaterialRef::default(),
			collection: Some(collection),
		}
	}

	/// Fold standard stick nodes into one collection (shared material, baked probe).
	pub fn merge_standard(nodes: impl IntoIterator<Item = Self>) -> Option<Self> {
		let nodes: Vec<Self> = nodes.into_iter().collect();
		let material = nodes.first()?.material.clone();
		let mut members = Vec::with_capacity(nodes.len());
		for node in &nodes {
			if let Some(existing) = &node.collection {
				members.extend(existing.members.iter().copied());
			} else {
				members.push(StickMember { geometry: node.geometry, placement: node.placement });
			}
		}
		if members.is_empty() {
			return None;
		}
		let collection = StickCollection::new(members).bake_bounds_from_members();
		Some(Self::collection(collection, Placement::IDENTITY).with_material(material))
	}

	fn probe(&self) -> StickLodProbe {
		if let Some(collection) = &self.collection {
			let (local_center, local_radius) = collection.center_and_extent();
			let world_center =
				self.placement.compose_child(Placement::new(local_center, 0.0)).translation;
			let scale = self.placement.scale.abs().max_element().max(1e-4);
			return StickLodProbe { center: world_center, extent: local_radius * scale };
		}
		StickLodProbe::from_stick(&self.placement, self.geometry)
	}

	fn glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		self.geometry.standard_glb_for_level(level)
	}

	fn member_glb(member: &StickMember, level: LodSceneLevel) -> Option<AssetPath> {
		member.geometry.standard_glb_for_level(level)
	}

	fn empty_scene() -> impl Scene + 'static {
		bsn! {
			Visibility::Inherited
		}
	}

	fn collection_multi_scene_merge(
		&self,
		collection: &StickCollection,
		level: LodSceneLevel,
	) -> Option<MultiSceneMerge> {
		let members = collection.members_for_level(level);
		if members.is_empty() {
			return None;
		}
		let mut parts = Vec::with_capacity(members.len());
		for member in &members {
			let asset = Self::member_glb(member, level)?;
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(member.placement)));
		}
		Some(MultiSceneMerge::new(parts))
	}

	fn collection_content(
		&self,
		collection: &StickCollection,
		level: LodSceneLevel,
	) -> Box<dyn Scene> {
		if matches!(level, LodSceneLevel::UltraLow) {
			return Box::new(Self::empty_scene());
		}
		if let Some(merge) = self.collection_multi_scene_merge(collection, level) {
			return Box::new(posed_foliage_multi_scene_merge(
				merge,
				pose(self.placement),
				self.material.clone(),
			));
		}
		Box::new((bsn! { NotShadowCaster }, scene_children(Vec::new())))
	}

	fn content_for_level(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		if let Some(collection) = &self.collection {
			return self.collection_content(collection, level);
		}
		match level {
			LodSceneLevel::UltraLow => Box::new(Self::empty_scene()),
			_ => match self.glb_for_level(level) {
				Some(asset) => Box::new((
					bsn! { NotShadowCaster },
					posed_material_asset_tier(
						Some(asset),
						pose(self.placement),
						Some(self.material.clone()),
					),
				)),
				None => Box::new(Self::empty_scene()),
			},
		}
	}
}

impl LodScene for StickNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		if self.collection.is_some() {
			let probe = self.probe();
			let distance = lod_ref.current_transform.translation.distance(probe.center);
			if distance <= STICK_COLLECTION_HIGH_METERS {
				return LodSceneCulls::None;
			}
			return cull_offset_bands_from_factor(
				distance,
				STICK_COLLECTION_HIGH_METERS,
				STICK_COLLECTION_MEDIUM_METERS,
				STICK_COLLECTION_LOW_METERS,
			);
		}
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.content_for_level(level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		let (center, extent) = if self.collection.is_some() {
			let probe = self.probe();
			(probe.center, probe.extent.max(1.0))
		} else {
			(
				crate::lod_band::placement_center(&self.placement),
				crate::lod_band::characteristic_extent_abs(&self.placement).max(1.0),
			)
		};
		let half = Vec3::splat(extent);
		Aabb3d::from_min_max(center - half, center + half)
	}
}
