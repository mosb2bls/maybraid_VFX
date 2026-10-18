//! Scene content for a [`FoliageNode`](super::FoliageNode).
//!
//! Single kits are one posed GLB. Kit collections pick
//! [`CollectionPresent::Merge`] (one [`MultiSceneMerge`]) or
//! [`CollectionPresent::Instance`] (posed siblings under the node placement).
//! Empty collections emit an empty parent, not a forced merge.

use bevy::light::NotShadowCaster;
use bevy::scene::prelude::{bsn, Scene};
use lod::scene::LodSceneLevel;
use scene_ref::{MultiSceneMerge, MultiScenePart};

use crate::assets::AssetPath;
use crate::foliage::collection::{CheapBallCollection, FrondCollection};
use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::present::CollectionPresent;
use crate::lod_host::{
	posed_foliage_asset_tier, posed_foliage_multi_scene_merge, posed_frond_asset_tier,
	posed_frond_multi_scene_merge,
};
use crate::scene_children::{pose, scene_children};

use super::FoliageNode;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::foliage::collection::{FrondCollection, FrondRun};
	use crate::placed::Placement;
	use bevy::prelude::{Entity, Transform, Vec3};
	use lod::lod_ref::LodRef;
	use lod::scene::LodScene;
	use lod::SceneChunk;

	fn one_run_collection() -> FrondCollection {
		FrondCollection::new([FrondRun::from_placements([Placement::frond_segment(
			Vec3::ZERO,
			Vec3::Y,
			1.0,
			0.02,
		)
		.expect("placement")])])
		.with_probe(Vec3::ZERO, 1.0)
	}

	fn dummy_lod() -> (Transform, bevy::math::bounding::Aabb3d) {
		(
			Transform::from_translation(Vec3::new(0.0, 2.0, 8.0)),
			bevy::math::bounding::Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
		)
	}

	#[test]
	fn empty_collection_is_empty_for_merge_and_instance() {
		let (camera, bounds) = dummy_lod();
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		for node in [
			FoliageNode::frond_collection(FrondCollection::new([]), Placement::IDENTITY),
			FoliageNode::frond_collection(FrondCollection::new([]), Placement::IDENTITY)
				.instanced(),
		] {
			assert!(matches!(
				node.scene_chunks_with_level(&lod_ref, LodSceneLevel::High),
				SceneChunk::Primitive { .. }
			));
		}
	}

	#[test]
	fn instanced_collection_with_members_is_still_one_host_chunk() {
		let (camera, bounds) = dummy_lod();
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		let node =
			FoliageNode::frond_collection(one_run_collection(), Placement::IDENTITY).instanced();
		assert_eq!(node.collection_present, CollectionPresent::Instance);
		assert!(matches!(
			node.scene_chunks_with_level(&lod_ref, LodSceneLevel::High),
			SceneChunk::Primitive { .. }
		));
	}
}

impl FoliageNode {
	pub(super) fn content_for_level(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		match &self.geometry {
			FoliageGeometry::CheapBall => Box::new((
				bsn! { NotShadowCaster },
				posed_foliage_asset_tier(
					self.standard_ball_glb_for_level(level),
					pose(self.placement),
					self.material.clone(),
				),
			)),
			FoliageGeometry::LayeredBall => Box::new(posed_foliage_asset_tier(
				self.standard_ball_glb_for_level(level),
				pose(self.placement),
				self.material.clone(),
			)),
			FoliageGeometry::StraightFrond | FoliageGeometry::StraightFrondSegment => {
				Box::new(posed_frond_asset_tier(
					self.standard_frond_glb_for_level(level),
					pose(self.placement),
					self.material.clone(),
				))
			}
			FoliageGeometry::FrondCollection(collection) => {
				self.frond_collection_content(collection, level)
			}
			FoliageGeometry::CheapBallCollection(collection) => {
				self.cheap_ball_collection_content(collection, level)
			}
		}
	}

	fn standard_ball_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match &self.geometry {
			FoliageGeometry::LayeredBall => {
				Some(FoliageGeometry::layered_ball_glb_for_level(level))
			}
			FoliageGeometry::CheapBall => Some(FoliageGeometry::cheap_ball_glb_for_level(level)),
			_ => None,
		}
	}

	fn standard_frond_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match &self.geometry {
			FoliageGeometry::StraightFrond => {
				Some(FoliageGeometry::straight_frond_glb_for_level(level))
			}
			FoliageGeometry::StraightFrondSegment => {
				Some(FoliageGeometry::straight_frond_segment_glb_for_level(level))
			}
			_ => None,
		}
	}

	fn frond_collection_content(
		&self,
		collection: &FrondCollection,
		level: LodSceneLevel,
	) -> Box<dyn Scene> {
		let members = collection.members_for_level(level);
		if members.is_empty() {
			return Box::new(scene_children(Vec::new()));
		}
		match self.collection_present {
			CollectionPresent::Merge => {
				let parts: Vec<MultiScenePart> = members
					.into_iter()
					.map(|member| {
						MultiScenePart::new(
							FoliageGeometry::frond_kit_glb_for_level(member.kit, level).scene_ref(),
							pose(member.placement),
						)
					})
					.collect();
				Box::new(posed_frond_multi_scene_merge(
					MultiSceneMerge::new(parts),
					pose(self.placement),
					self.material.clone(),
				))
			}
			CollectionPresent::Instance => {
				let children: Vec<Box<dyn Scene>> = members
					.into_iter()
					.map(|member| {
						Box::new(posed_frond_asset_tier(
							Some(FoliageGeometry::frond_kit_glb_for_level(member.kit, level)),
							pose(self.placement.compose_child(member.placement)),
							self.material.clone(),
						)) as Box<dyn Scene>
					})
					.collect();
				Box::new(scene_children(children))
			}
		}
	}

	fn cheap_ball_collection_content(
		&self,
		collection: &CheapBallCollection,
		level: LodSceneLevel,
	) -> Box<dyn Scene> {
		let placements = collection.placements_for_level(level);
		if placements.is_empty() {
			return Box::new((bsn! { NotShadowCaster }, scene_children(Vec::new())));
		}
		let asset = FoliageGeometry::cheap_ball_glb_for_level(level);
		match self.collection_present {
			CollectionPresent::Merge => {
				let parts: Vec<MultiScenePart> = placements
					.into_iter()
					.map(|placement| MultiScenePart::new(asset.scene_ref(), pose(placement)))
					.collect();
				Box::new(posed_foliage_multi_scene_merge(
					MultiSceneMerge::new(parts),
					pose(self.placement),
					self.material.clone(),
				))
			}
			CollectionPresent::Instance => {
				let children: Vec<Box<dyn Scene>> = placements
					.into_iter()
					.map(|placement| {
						Box::new((
							bsn! { NotShadowCaster },
							posed_foliage_asset_tier(
								Some(asset),
								pose(self.placement.compose_child(placement)),
								self.material.clone(),
							),
						)) as Box<dyn Scene>
					})
					.collect();
				Box::new(scene_children(children))
			}
		}
	}
}
