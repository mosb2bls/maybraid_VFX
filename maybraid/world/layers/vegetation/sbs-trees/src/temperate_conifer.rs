//! **Temperate Conifer** — sparse fronded Friend's Conifer variant ([#238](https://github.com/ramate-io/maybraid/issues/238),
//! [RFC-183 §3.1.7.15](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/15-temperate-conifer/README.md)).
//!
//! [`TemperateConiferParams::build`] applies the temperate preset, grows the ball-stick chain once
//! into [`TemperateConifer`], which implements [`VegetationComponents`]. Sticks reuse Northern /
//! Liam banding; High/Medium/Low foliage structurally samples joints, then packs one
//! [`FrondCollection`] per branch ring (collections scale out — no mass proxy).
//!
//! [`TemperateConifer::unit_from_num`] / [`TemperateConiferParams::into_unit_from_num`]
//! apply the temperate preset, normalize to unit height, and key layout noise by a
//! variant index. Emission folds sticks into a collection; frond collections stay
//! separate nodes.

mod foliage;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{parse_unit_range, UnitRange};
use sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use sbs_geometry::{align_frond_direction, FrondCrownShape};
use sbs_geometry::{
	liams_stalk_tip_from_chain, sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands,
	BallStickChain, FriendsConiferChain, FriendsConiferSbs,
};
use vegetation_components::{
	stick_material_ref, FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode,
	StructuralLod, VegetationComponents,
};

use crate::conifer_canopy_apex::{sample_apex_canopy_spawn, DEFAULT_APEX_CANOPY_SPAWN_FRACTION};
use crate::northern_conifer::stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};
use crate::palm_tree::world_space_frond_shape;
use crate::storybook_tree::merge_kit_sticks;
use foliage::{branch_direction, frond_shape_for_joint};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

/// High: denser structural joint samples before ring packing (~+15% vs prior 24×8).
const HIGH_JOINT_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(28, 9);
/// Medium: ~30% denser than prior 15×4 joint samples.
const MEDIUM_JOINT_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(20, 4);
/// Low: coarser joint samples; frond collections scale out (no mass proxy).
const LOW_JOINT_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(10, 3);

/// [`FriendsConiferSbs`] with Temperate Conifer limb/ray defaults (clap `flatten` base).
#[derive(Clone, Debug, PartialEq, Args)]
#[command(rename_all = "kebab-case")]
pub struct TemperateConiferGeometry {
	#[command(flatten)]
	pub inner: FriendsConiferSbs,
}

impl Default for TemperateConiferGeometry {
	fn default() -> Self {
		let mut inner = FriendsConiferSbs::default();
		inner.apply_temperate_preset();
		Self { inner }
	}
}

impl std::ops::Deref for TemperateConiferGeometry {
	type Target = FriendsConiferSbs;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl From<FriendsConiferSbs> for TemperateConiferGeometry {
	fn from(inner: FriendsConiferSbs) -> Self {
		Self { inner }
	}
}

/// Authoring / CLI parameters for Temperate Conifer.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct TemperateConiferParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: TemperateConiferGeometry,

	/// Uniform world scale applied to each joint [`FrondCrownShape`].
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub frond_world_scale: f32,

	/// Fronds placed per ball-stick joint (slightly denser than RFC `1..2`).
	#[arg(
		long = "fronds-per-joint",
		default_value = "2..3",
		value_parser = parse_unit_range,
		value_name = "MIN..MAX",
		help_heading = "Foliage"
	)]
	pub fronds_per_joint: UnitRange,

	/// Frond spine length as a fraction of stalk height (RFC `0.035..0.07`).
	#[arg(
		long = "frond-length-fraction",
		default_value = "0.035..0.07",
		value_parser = parse_unit_range,
		value_name = "MIN..MAX",
		help_heading = "Foliage"
	)]
	pub frond_length_fraction: UnitRange,

	/// Fraction of joints that receive fronds (sparse dryland &lt; 1.0).
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub frond_spawn_fraction: f32,

	/// Fraction of trees that spawn a downward frond crown at the stalk tip (noise-gated).
	#[arg(long, default_value_t = DEFAULT_APEX_CANOPY_SPAWN_FRACTION, help_heading = "Foliage")]
	pub apex_canopy_spawn_fraction: f32,
}

impl Default for TemperateConiferParams {
	fn default() -> Self {
		Self {
			geometry: TemperateConiferGeometry::default(),
			frond_world_scale: 1.0,
			fronds_per_joint: UnitRange::new(2.0, 3.0),
			frond_length_fraction: UnitRange::new(0.035, 0.07),
			frond_spawn_fraction: 1.0,
			apex_canopy_spawn_fraction: DEFAULT_APEX_CANOPY_SPAWN_FRACTION,
		}
	}
}

impl TemperateConiferParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> TemperateConifer {
		TemperateConifer::from_params(self)
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	///
	/// Applies the temperate preset first so world size is the post-preset height.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		geometry.inner.apply_temperate_preset();
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.inner.scale.stalk_height = 1.0;
		if let Some(radius) = geometry.inner.scale.stalk_base_radius {
			geometry.inner.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.inner.canopy_noise.seed = num as i32;
		geometry.inner.anchor_perturbation.noise.seed = num as i32;
		(
			Self {
				geometry,
				frond_world_scale: self.frond_world_scale,
				fronds_per_joint: self.fronds_per_joint,
				frond_length_fraction: self.frond_length_fraction,
				frond_spawn_fraction: self.frond_spawn_fraction,
				apex_canopy_spawn_fraction: self.apex_canopy_spawn_fraction,
			},
			size,
		)
	}
}

/// Built Temperate Conifer: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct TemperateConifer {
	pub geometry: TemperateConiferGeometry,
	pub chain: BallStickChain<FriendsConiferChain>,
	pub frond_world_scale: f32,
	pub fronds_per_joint: UnitRange,
	pub frond_length_fraction: UnitRange,
	pub frond_spawn_fraction: f32,
	pub apex_canopy_spawn_fraction: f32,
}

impl TemperateConifer {
	pub fn from_params(params: &TemperateConiferParams) -> Self {
		let mut geometry = params.geometry.clone();
		geometry.inner.apply_temperate_preset();
		Self {
			chain: geometry.inner.build_chain(),
			geometry,
			frond_world_scale: params.frond_world_scale,
			fronds_per_joint: params.fronds_per_joint,
			frond_length_fraction: params.frond_length_fraction,
			frond_spawn_fraction: params.frond_spawn_fraction,
			apex_canopy_spawn_fraction: params.apex_canopy_spawn_fraction,
		}
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&TemperateConiferParams::unit_from_num(num))
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn height(&self) -> f32 {
		self.geometry.height()
	}

	fn ring_key(&self, y: f32) -> i32 {
		let h = self.height().max(1e-6);
		let first = self.geometry.rings.height_range.start;
		let spacing = self.geometry.rings.spacing.max(1e-4);
		let z_frac = (y / h).clamp(0.0, 1.0);
		((z_frac - first) / spacing).round() as i32
	}

	/// Spawn-gated joint candidates (pre structural sample).
	fn joint_candidates(&self) -> Vec<(usize, Vec3)> {
		self.chain
			.nodes_with_hysteresis_enumerated()
			.filter_map(|(node_idx, node, _)| {
				if !mix_seed_below_fraction(node_idx, node.position, self.frond_spawn_fraction) {
					return None;
				}
				Some((node_idx, node.position))
			})
			.collect()
	}

	fn sampled_joints(&self, bands: AzimuthHeightBands) -> Vec<(usize, Vec3)> {
		let candidates = self.joint_candidates();
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.1, bands)
			.into_iter()
			.map(|s| *s.item)
			.collect()
	}

	fn rotated_frond_runs(origin: Vec3, rotation: Quat, shape: &FrondCrownShape) -> Vec<FrondRun> {
		shape
			.frond_runs_at(Vec3::ZERO)
			.into_iter()
			.filter_map(|run| {
				let placements: Vec<Placement> = run
					.into_iter()
					.filter_map(|seg| {
						let start = origin + rotation * seg.start;
						let dir = rotation * seg.direction;
						Placement::frond_segment(start, dir, seg.length, seg.width)
					})
					.collect();
				if placements.is_empty() {
					None
				} else {
					Some(FrondRun::from_placements(placements))
				}
			})
			.collect()
	}

	fn apex_frond_placement(&self) -> Option<(Vec3, Quat, FrondCrownShape)> {
		let tip = liams_stalk_tip_from_chain(&self.chain);
		if !sample_apex_canopy_spawn(
			&self.geometry.canopy_noise,
			&tip,
			self.apex_canopy_spawn_fraction,
		) {
			return None;
		}
		let h = self.height();
		let scale = self.frond_world_scale.max(1e-8);
		let seed = self.geometry.canopy_noise.seed.wrapping_add(0xC1A0);
		let frond_count = 3 + ((seed as u32) % 2);
		let local = FrondCrownShape {
			frond_count,
			length: (0.065 * h) / scale,
			width: (0.012 * h) / scale,
			droop: 0.28,
			arch_lift: 0.06,
			twist: 0.12,
			leaflet_count: 6,
			spine_segments: 3,
			shoot_half_radius: 0.008,
			rachis_half_thickness: 0.004,
			leaflet_length_scale: 2.4,
			downward_tilt_radians: 0.42,
			outward_spread_radians: 0.55,
			emission_lift_radians: 0.05,
			seed,
		};
		Some((
			tip.position,
			align_frond_direction(Vec3::NEG_Y),
			world_space_frond_shape(local, self.frond_world_scale),
		))
	}

	/// One [`FrondCollection`] per branch ring from structurally sampled joints.
	fn ring_frond_collections(&self, bands: AzimuthHeightBands) -> Vec<FoliageNode> {
		let scale = self.frond_world_scale;
		let mut rings: BTreeMap<i32, Vec<FrondRun>> = BTreeMap::new();
		for (node_idx, position) in self.sampled_joints(bands) {
			let Some(node) = self.chain.nodes.get(node_idx) else {
				continue;
			};
			let local = frond_shape_for_joint(
				&self.geometry.inner,
				scale,
				node_idx,
				node,
				&self.fronds_per_joint,
				&self.frond_length_fraction,
			);
			let shape = world_space_frond_shape(local, scale);
			let rotation = align_frond_direction(branch_direction(&self.chain, node_idx, node));
			let key = self.ring_key(position.y);
			rings
				.entry(key)
				.or_default()
				.extend(Self::rotated_frond_runs(position, rotation, &shape));
		}
		if let Some((tip, rotation, shape)) = self.apex_frond_placement() {
			let key = self.ring_key(tip.y).saturating_add(1);
			rings
				.entry(key)
				.or_default()
				.extend(Self::rotated_frond_runs(tip, rotation, &shape));
		}
		rings
			.into_values()
			.filter(|runs| !runs.is_empty())
			.map(|runs| {
				FoliageNode::frond_collection(
					FrondCollection::new(runs).bake_bounds_from_runs(),
					Placement::IDENTITY,
				)
			})
			.collect()
	}
}

impl VegetationComponents for TemperateConifer {
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
		let bands = match level {
			LodSceneLevel::High => HIGH_JOINT_BANDS,
			LodSceneLevel::Medium => MEDIUM_JOINT_BANDS,
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => LOW_JOINT_BANDS,
		};
		Layers::from_free(self.ring_frond_collections(bands))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.height(),
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

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = TemperateConifer::unit_from_num(3);
		let b = TemperateConifer::unit_from_num(3);
		let c = TemperateConifer::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = TemperateConiferParams::default();
		params.geometry.inner.scale.stalk_height = 8.0;
		params.geometry.inner.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.inner.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_collection() -> Result<()> {
		let tree = TemperateConifer::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!foliage.is_empty());
		assert!(foliage.iter().all(|n| n.geometry.is_frond_collection()));
		Ok(())
	}
}
