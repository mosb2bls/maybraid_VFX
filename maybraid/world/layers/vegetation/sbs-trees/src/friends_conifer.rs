//! **Friend's Conifer** — log-profile conifer with plane-splay foliage ([#236](https://github.com/ramate-io/maybraid/issues/236),
//! [RFC-183 §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
//!
//! [`FriendsConiferParams::build`] grows the ball-stick chain once into [`FriendsConifer`], which
//! implements [`VegetationComponents`]. Sticks/foliage reuse Northern banding (`FriendsConiferChain`
//! is [`LiamsConiferChain`](sbs_geometry::LiamsConiferChain)); High/Medium/Low emit cheap-ball
//! joint clusters sized by splay radius (historical plane-splay mesh is not used under VC).
//!
//! [`FriendsConifer::unit_from_num`] / [`FriendsConiferParams::into_unit_from_num`]
//! normalize to unit height and key layout noise by a variant index so many plants
//! share one archetypal mesh (world size goes on [`Placement`] scale). Emission
//! folds sticks and cheap balls into collections.

pub mod canopy;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{BallStickChain, FriendsConiferChain, FriendsConiferSbs};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::conifer_canopy_apex::{
	DEFAULT_APEX_CANOPY_SPAWN_FRACTION, FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
use crate::northern_conifer::canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};
use crate::northern_conifer::stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};
use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
pub use canopy::{
	FRIENDS_SPLAY_CORE_RADIUS, FRIENDS_SPLAY_LEAF_DISC_RADIUS,
	FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

/// Authoring / CLI parameters for Friend's Conifer.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct FriendsConiferParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: FriendsConiferSbs,

	/// Plane-splay world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub splay_radius_fraction_of_height: f32,

	/// Fraction of ball-stick joints that receive foliage (1.0 = all joints).
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub splay_spawn_fraction: f32,

	/// Fraction of trees that spawn one apex ball at the stalk crown.
	#[arg(long, default_value_t = DEFAULT_APEX_CANOPY_SPAWN_FRACTION, help_heading = "Foliage")]
	pub apex_canopy_spawn_fraction: f32,

	/// Apex ball world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub apex_ball_radius_fraction_of_height: f32,
}

impl Default for FriendsConiferParams {
	fn default() -> Self {
		Self {
			geometry: FriendsConiferSbs::default(),
			splay_radius_fraction_of_height: FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
			splay_spawn_fraction: 1.0,
			apex_canopy_spawn_fraction: DEFAULT_APEX_CANOPY_SPAWN_FRACTION,
			apex_ball_radius_fraction_of_height: FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		}
	}
}

impl FriendsConiferParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> FriendsConifer {
		FriendsConifer::from_params(self)
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.stalk_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.canopy_noise.seed = num as i32;
		geometry.anchor_perturbation.noise.seed = num as i32;
		(
			Self {
				geometry,
				splay_radius_fraction_of_height: self.splay_radius_fraction_of_height,
				splay_spawn_fraction: self.splay_spawn_fraction,
				apex_canopy_spawn_fraction: self.apex_canopy_spawn_fraction,
				apex_ball_radius_fraction_of_height: self.apex_ball_radius_fraction_of_height,
			},
			size,
		)
	}
}

/// Built Friend's Conifer: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct FriendsConifer {
	pub geometry: FriendsConiferSbs,
	pub chain: BallStickChain<FriendsConiferChain>,
	pub splay_radius_fraction_of_height: f32,
	pub splay_spawn_fraction: f32,
	pub apex_canopy_spawn_fraction: f32,
	pub apex_ball_radius_fraction_of_height: f32,
}

impl FriendsConifer {
	pub fn from_params(params: &FriendsConiferParams) -> Self {
		Self {
			chain: params.geometry.build_chain(),
			geometry: params.geometry.clone(),
			splay_radius_fraction_of_height: params.splay_radius_fraction_of_height,
			splay_spawn_fraction: params.splay_spawn_fraction,
			apex_canopy_spawn_fraction: params.apex_canopy_spawn_fraction,
			apex_ball_radius_fraction_of_height: params.apex_ball_radius_fraction_of_height,
		}
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&FriendsConiferParams::unit_from_num(num))
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn splay_radius_world(&self) -> f32 {
		self.geometry.height() * self.splay_radius_fraction_of_height
	}

	fn apex_radius_world(&self) -> f32 {
		self.geometry.height() * self.apex_ball_radius_fraction_of_height
	}
}

impl VegetationComponents for FriendsConifer {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium(&self.chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(stick_material_ref())).collect();
		Layers::from_free(merge_kit_sticks(nodes))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let splay_r = self.splay_radius_world();
		let apex_r = self.apex_radius_world();
		let nodes = match level {
			LodSceneLevel::High => foliage_nodes_banded(
				&self.chain,
				HIGH_FOLIAGE_BANDS,
				splay_r,
				self.splay_spawn_fraction,
				self.apex_canopy_spawn_fraction,
				apex_r,
			),
			LodSceneLevel::Medium => foliage_nodes_medium(
				&self.chain,
				splay_r,
				self.splay_spawn_fraction,
				self.apex_canopy_spawn_fraction,
				apex_r,
			),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low(
				&self.chain,
				splay_r,
				self.splay_spawn_fraction,
				self.apex_canopy_spawn_fraction,
				apex_r,
			),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(leaf_material_ref())).collect();
		Layers::from_free(merge_cheap_ball_foliage(nodes))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.geometry.height(),
			)
			.with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use vegetation_components::FoliageGeometry;

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = FriendsConifer::unit_from_num(3);
		let b = FriendsConifer::unit_from_num(3);
		let c = FriendsConifer::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = FriendsConiferParams::default();
		params.geometry.scale.stalk_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_and_cheap_ball_collections() -> Result<()> {
		let tree = FriendsConifer::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(foliage.len(), 1);
		assert!(matches!(foliage[0].geometry, FoliageGeometry::CheapBallCollection(_)));
		Ok(())
	}
}
