//! **Palm Crown** — stacked frond rings as [`VegetationComponents`].
//!
//! High / Medium emit one [`FrondCollection`] per frond with a multi-segment rachis run;
//! fine-phase collection merge does Medium decimation. Collections stay per-frond so
//! UltraLow merge cannot chord the whole crown; the LOD probe is the parent crown
//! (same unit as structural LOD). Low / UltraLow emit a shared five-chord star
//! ([`PalmCrownParams::unit_low_star`]) — not layered / cheap balls. Probe and
//! collection nodes bake at build so produce / grove emit do not walk the crown AABB.

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::FrondCrownShape;
use vegetation_components::{
	FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::palm_tree::{low_star_collection_nodes, LOW_STAR_FROND_COUNT};

/// Per-ring seed salt mixed into foliage noise (Date, Waialea, Palm Bush, PalmCrown).
pub const FROND_RING_SEED_SALT: i32 = 17;

/// High when `distance / crown_radius ≤` this (rachis / full frond topology).
const PALM_CROWN_STRUCTURAL_HIGH_FACTOR: f32 = 36.0;
/// Medium outer edge (fine-phase merge; same frond IR as High).
const PALM_CROWN_STRUCTURAL_MEDIUM_FACTOR: f32 = 54.0;
/// Low outer edge (shared five-chord star).
const PALM_CROWN_STRUCTURAL_LOW_FACTOR: f32 = 100.0;

/// Authoring / CLI parameters for a palm crown (standalone or stacked rings).
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct PalmCrownParams {
	/// Number of stacked frond rings along +Y.
	#[arg(long, default_value_t = 3)]
	pub ring_count: u32,

	/// Vertical spacing (m) between successive ring anchors.
	#[arg(long, default_value_t = 0.14)]
	pub ring_spacing: f32,

	#[command(flatten, next_help_heading = "Frond Crown")]
	pub shape: FrondCrownShape,
}

impl Default for PalmCrownParams {
	fn default() -> Self {
		Self {
			ring_count: 3,
			ring_spacing: 0.14,
			shape: FrondCrownShape {
				// Palmier than the single-ring mesh default; sparse fronds + short rachis.
				frond_count: 5,
				length: 1.6,
				width: 0.16,
				droop: 0.55,
				arch_lift: 0.28,
				twist: 0.55,
				leaflet_count: 16,
				// Two chords per frond — enough for droop without dense rachis tessellation.
				spine_segments: 2,
				downward_tilt_radians: 0.55,
				outward_spread_radians: 1.4,
				emission_lift_radians: 0.32,
				..FrondCrownShape::default()
			},
		}
	}
}

/// Mid-band date-palm frond length as a fraction of stalk height `H` (RFC `0.6`–`0.8`).
pub(crate) const DATE_PALM_FROND_LENGTH_FRACTION: f32 = 0.7;
/// Date-palm rachis width as a fraction of `H`.
pub(crate) const DATE_PALM_FROND_WIDTH_FRACTION: f32 = 0.07;
/// Mid-band palm-bush / Waialea frond length as a fraction of `H` (RFC `0.25`–`0.40`).
pub(crate) const DETAIL_FROND_LENGTH_FRACTION: f32 = 0.325;
/// Palm-bush / Waialea rachis width as a fraction of `H`.
pub(crate) const DETAIL_FROND_WIDTH_FRACTION: f32 = 0.05;

impl PalmCrownParams {
	pub fn new(ring_count: u32, ring_spacing: f32, shape: FrondCrownShape) -> Self {
		Self { ring_count, ring_spacing, shape }
	}

	/// Characteristic authored size before unit normalize (Placement world scale).
	///
	/// `max(length, width * fronds, ring-stack height)`.
	pub fn characteristic_size(&self) -> f32 {
		let rings = self.ring_count.max(1).saturating_sub(1) as f32;
		let stack = rings * self.ring_spacing.max(0.0);
		let frond_span = self.shape.width.max(0.0) * self.shape.frond_count.max(1) as f32;
		self.shape.length.max(frond_span).max(stack).max(1e-4)
	}

	/// World-meter tree-top crown metrics for stalk height `H` (date-palm length band).
	///
	/// Pass into [`Self::into_unit_full_from_num`] so Placement scale restores meters after
	/// unit normalize.
	pub fn authored_full_for_height(height: f32) -> Self {
		let h = height.max(1e-6);
		Self {
			ring_count: 3,
			ring_spacing: 0.14,
			shape: FrondCrownShape {
				length: (DATE_PALM_FROND_LENGTH_FRACTION * h).max(1e-4),
				width: (DATE_PALM_FROND_WIDTH_FRACTION * h).max(1e-6),
				..Self::default().shape
			},
		}
	}

	/// World-meter understory crown metrics for plant height `H` (palm-bush / Waialea band).
	pub fn authored_detail_for_height(height: f32) -> Self {
		let h = height.max(1e-6);
		Self {
			ring_count: 2,
			ring_spacing: 0.1,
			shape: FrondCrownShape {
				length: (DETAIL_FROND_LENGTH_FRACTION * h).max(1e-4),
				width: (DETAIL_FROND_WIDTH_FRACTION * h).max(1e-6),
				..Self::default().shape
			},
		}
	}

	/// Unit full crown + Placement world scale for height `H`.
	pub fn unit_full_for_height_from_num(height: f32, num: u32) -> (Self, f32) {
		Self::authored_full_for_height(height).into_unit_full_from_num(num)
	}

	/// Unit detail crown + Placement world scale for height `H`.
	pub fn unit_detail_for_height_from_num(height: f32, num: u32) -> (Self, f32) {
		Self::authored_detail_for_height(height).into_unit_detail_from_num(num)
	}

	fn apply_full_archetype(&mut self) {
		// A few stacked rings + articulated rachis (not the sparse mesh default).
		self.ring_count = 6;
		self.shape.frond_count = 10;
		self.shape.spine_segments = 9;
		self.shape.leaflet_count = 16;
		self.shape.droop = 0.55;
		self.shape.arch_lift = 0.28;
		self.shape.twist = 0.55;
		self.shape.downward_tilt_radians = 0.55;
		self.shape.outward_spread_radians = 1.4;
		self.shape.emission_lift_radians = 0.32;
	}

	fn apply_detail_archetype(&mut self) {
		self.ring_count = 3;
		self.shape.frond_count = 6;
		self.shape.spine_segments = 5;
		self.shape.leaflet_count = 12;
		self.shape.droop = 0.5;
		self.shape.arch_lift = 0.22;
		self.shape.twist = 0.45;
		self.shape.downward_tilt_radians = 0.5;
		self.shape.outward_spread_radians = 1.25;
		self.shape.emission_lift_radians = 0.28;
	}

	fn apply_low_star_archetype(&mut self) {
		self.ring_count = 1;
		self.ring_spacing = 0.0;
		self.shape.frond_count = LOW_STAR_FROND_COUNT;
		self.shape.spine_segments = 1;
		self.shape.leaflet_count = 2;
		self.shape.droop = (self.shape.length * 0.85).max(1e-4);
		self.shape.arch_lift = self.shape.length * 0.2;
		self.shape.twist = 0.0;
		self.shape.downward_tilt_radians = 0.70;
		self.shape.outward_spread_radians = 0.0;
		self.shape.emission_lift_radians = 0.0;
		self.shape.seed = 0;
	}

	fn normalize_to_unit(&mut self) -> f32 {
		let size = self.characteristic_size();
		let inv = 1.0 / size;
		self.shape.length *= inv;
		self.shape.width *= inv;
		self.shape.droop *= inv;
		self.shape.arch_lift *= inv;
		self.shape.shoot_half_radius *= inv;
		self.shape.rachis_half_thickness *= inv;
		self.ring_spacing *= inv;
		size
	}

	/// Proper tree-top crown archetype with unit characteristic size, keyed by `num` (seed).
	///
	/// The mesh is ~unit sized; for grove Placement scale use
	/// [`Self::unit_full_for_height_from_num`] (or [`Self::into_unit_full_from_num`] with
	/// authored world meters).
	pub fn unit_full_from_num(num: u32) -> Self {
		Self::unit_full_for_height_from_num(1.6 / DATE_PALM_FROND_LENGTH_FRACTION, num).0
	}

	/// Lighter understory crown archetype with unit characteristic size, keyed by `num`.
	///
	/// The mesh is ~unit sized; for grove Placement scale use
	/// [`Self::unit_detail_for_height_from_num`].
	pub fn unit_detail_from_num(num: u32) -> Self {
		Self::unit_detail_for_height_from_num(1.0 / DETAIL_FROND_LENGTH_FRACTION, num).0
	}

	/// Apply full tree-top topology, normalize authored metrics to unit, key by `num`.
	///
	/// Returns `(unit_params, world_size)` for [`Placement`] scale.
	pub fn into_unit_full_from_num(mut self, num: u32) -> (Self, f32) {
		self.apply_full_archetype();
		let size = self.normalize_to_unit();
		self.shape.seed = num as i32;
		(self, size)
	}

	/// Apply understory topology, normalize authored metrics to unit, key by `num`.
	pub fn into_unit_detail_from_num(mut self, num: u32) -> (Self, f32) {
		self.apply_detail_archetype();
		let size = self.normalize_to_unit();
		self.shape.seed = num as i32;
		(self, size)
	}

	/// Shared Low crown: one ring, five chords, seed 0. Not keyed by `num`.
	pub fn unit_low_star() -> Self {
		let mut params = Self::default();
		params.apply_low_star_archetype();
		params.normalize_to_unit();
		params
	}

	/// Tree-local ring anchors stacked along +Y from the origin.
	pub fn ring_anchors(&self) -> Vec<Vec3> {
		let n = self.ring_count.max(1);
		(0..n)
			.map(|i| Vec3::new(0.0, i as f32 * self.ring_spacing.max(0.0), 0.0))
			.collect()
	}

	/// Shape for ring `index`, seed-salted so stacked rings do not clone.
	pub fn ring_shape(&self, index: u32) -> FrondCrownShape {
		FrondCrownShape {
			seed: self.shape.seed.wrapping_add(index as i32 * FROND_RING_SEED_SALT),
			..self.shape.clone()
		}
	}

	/// Grow ring anchors once for presentation / LOD emission.
	pub fn build(&self) -> PalmCrown {
		PalmCrown::from_params(self)
	}
}

/// Built palm crown: params, ring anchors, plus baked unit probe / collection nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct PalmCrown {
	pub ring_count: u32,
	pub ring_spacing: f32,
	pub shape: FrondCrownShape,
	pub anchors: Vec<Vec3>,
	structural_lod: StructuralLod,
	high_nodes: Vec<FoliageNode>,
	low_nodes: Vec<FoliageNode>,
}

impl PalmCrown {
	pub fn from_params(params: &PalmCrownParams) -> Self {
		let mut crown = Self {
			ring_count: params.ring_count,
			ring_spacing: params.ring_spacing,
			shape: params.shape.clone(),
			anchors: params.ring_anchors(),
			structural_lod: StructuralLod::default(),
			high_nodes: Vec::new(),
			low_nodes: Vec::new(),
		};
		let (min, max) = crown.crown_aabb();
		let center = (min + max) * 0.5;
		let half = (max - min) * 0.5;
		let radius = half.x.max(half.y).max(half.z).max(1e-3);
		crown.structural_lod = StructuralLod::new(center, radius).with_factors(
			PALM_CROWN_STRUCTURAL_HIGH_FACTOR,
			PALM_CROWN_STRUCTURAL_MEDIUM_FACTOR,
			PALM_CROWN_STRUCTURAL_LOW_FACTOR,
		);
		crown.high_nodes = crown.frond_nodes_at(center, radius, false);
		let anchor = crown.anchors.first().copied().unwrap_or(Vec3::ZERO);
		crown.low_nodes = low_star_collection_nodes(
			anchor,
			crown.shape.length,
			crown.shape.width,
			center,
			radius,
		);
		crown
	}

	fn ring_shape(&self, index: u32) -> FrondCrownShape {
		FrondCrownShape {
			seed: self.shape.seed.wrapping_add(index as i32 * FROND_RING_SEED_SALT),
			..self.shape.clone()
		}
	}

	/// One [`FrondCollection`] per frond (single run).
	///
	/// Ring-wide collections make UltraLow merge chord the whole crown; per-frond
	/// collections keep merge extent ≈ rachis length. LOD probe is the parent crown,
	/// not the run AABB.
	///
	/// When `collapse_rachis`, each multi-segment spine becomes a single base→tip chord.
	fn frond_nodes_at(
		&self,
		probe_center: Vec3,
		probe_radius: f32,
		collapse_rachis: bool,
	) -> Vec<FoliageNode> {
		let mut nodes = Vec::new();
		for (index, anchor) in self.anchors.iter().enumerate() {
			let shape = self.ring_shape(index as u32);
			for run in shape.frond_runs_at(*anchor) {
				let placements: Vec<Placement> = run
					.into_iter()
					.filter_map(|seg| {
						Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
					})
					.collect();
				if placements.is_empty() {
					continue;
				}
				let mut frond_run = FrondRun::from_placements(placements);
				if collapse_rachis {
					let Some(chord) = frond_run.collapse_to_chord(1.0) else {
						continue;
					};
					frond_run = FrondRun::new([chord]);
				}
				nodes.push(FoliageNode::frond_collection(
					FrondCollection::new([frond_run]).with_probe(probe_center, probe_radius),
					Placement::IDENTITY,
				));
			}
		}
		nodes
	}

	/// AABB of High rachis polylines (origin + droop extents).
	fn crown_aabb(&self) -> (Vec3, Vec3) {
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		let mut any = false;
		for (index, anchor) in self.anchors.iter().enumerate() {
			let shape = self.ring_shape(index as u32);
			for run in shape.frond_runs_at(*anchor) {
				for seg in run {
					let tip = seg.start + seg.direction * seg.length;
					let half_w = seg.width * 0.5;
					for p in [seg.start, tip] {
						min = min.min(p - Vec3::splat(half_w));
						max = max.max(p + Vec3::splat(half_w));
						any = true;
					}
				}
			}
		}
		if !any {
			let r = self.shape.length.max(0.5);
			return (Vec3::splat(-r), Vec3::new(r, r, r));
		}
		(min, max)
	}
}

impl VegetationComponents for PalmCrown {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			// Medium keeps High topology; FrondCollection fine-phase merge thins runs.
			LodSceneLevel::High | LodSceneLevel::Medium => {
				Layers::from_free(self.high_nodes.clone())
			}
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => Layers::from_free(self.low_nodes.clone()),
		}
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(self.structural_lod)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::scene::LodSceneLevel;

	fn crown(seed: i32) -> PalmCrownParams {
		PalmCrownParams {
			shape: FrondCrownShape {
				seed,
				frond_count: 6,
				spine_segments: 4,
				..PalmCrownParams::default().shape
			},
			..PalmCrownParams::default()
		}
	}

	#[test]
	fn ring_anchors_stack_along_y() -> Result<()> {
		let params = PalmCrownParams { ring_count: 3, ring_spacing: 0.2, ..crown(1) };
		let anchors = params.ring_anchors();
		assert_eq!(anchors.len(), 3);
		assert_eq!(anchors[0], Vec3::ZERO);
		assert_eq!(anchors[1], Vec3::new(0.0, 0.2, 0.0));
		assert_eq!(anchors[2], Vec3::new(0.0, 0.4, 0.0));
		Ok(())
	}

	#[test]
	fn high_emits_one_collection_per_frond() -> Result<()> {
		let built = crown(3).build();
		let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		// 3 rings × 6 fronds (test shape).
		assert_eq!(nodes.len(), built.anchors.len() * 6);
		let collection = nodes[0].geometry.as_frond_collection().expect("collection");
		assert_eq!(collection.runs.len(), 1);
		assert_eq!(collection.runs[0].segments.len(), 4);
		let (center, extent) = collection.center_and_extent();
		let probe = built.structural_lod().expect("probe");
		assert!((center - probe.center).length() < 1e-4);
		assert!((extent - probe.tree_radius).abs() < 1e-4);
		let baked = FrondCollection::new(collection.runs.clone()).bake_bounds_from_runs();
		let (_, baked_extent) = baked.center_and_extent();
		assert!(
			extent > baked_extent,
			"crown probe {extent} should exceed one-frond AABB {baked_extent}"
		);
		Ok(())
	}

	#[test]
	fn medium_keeps_high_rachis_low_is_shared_star() -> Result<()> {
		let built = crown(5).build();
		let high = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		let medium = built.foliage_nodes_for_level(LodSceneLevel::Medium).flatten();
		assert_eq!(medium.len(), high.len());
		let high_segs = high[0].geometry.as_frond_collection().expect("high collection").runs[0]
			.segments
			.len();
		assert!(high_segs > 1);
		let medium_segs = medium[0].geometry.as_frond_collection().expect("medium collection").runs
			[0]
		.segments
		.len();
		assert_eq!(medium_segs, high_segs);

		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), LOW_STAR_FROND_COUNT as usize);
		assert!(low.iter().all(|n| n.geometry.is_frond_collection()));
		for node in &low {
			let collection = node.geometry.as_frond_collection().expect("star");
			assert_eq!(collection.runs.len(), 1);
			assert_eq!(collection.runs[0].segments.len(), 1);
		}

		let ultra = built.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten();
		assert_eq!(ultra.len(), LOW_STAR_FROND_COUNT as usize);
		assert!(ultra.iter().all(|n| n.geometry.is_frond_collection()));
		Ok(())
	}

	#[test]
	fn unit_low_star_is_shared_unit_footprint() -> Result<()> {
		let a = PalmCrownParams::unit_low_star();
		let b = PalmCrownParams::unit_low_star();
		assert_eq!(a, b);
		assert_eq!(a.shape.seed, 0);
		assert_eq!(a.ring_count, 1);
		assert_eq!(a.shape.frond_count, LOW_STAR_FROND_COUNT);
		assert_eq!(a.shape.spine_segments, 1);
		assert!((a.characteristic_size() - 1.0).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn structural_bands_are_palm_crown_local() -> Result<()> {
		let built = crown(0).build();
		let probe = built.structural_lod().expect("probe");
		assert_eq!(probe.high_factor, PALM_CROWN_STRUCTURAL_HIGH_FACTOR);
		assert_eq!(probe.medium_factor, PALM_CROWN_STRUCTURAL_MEDIUM_FACTOR);
		assert_eq!(probe.low_factor, PALM_CROWN_STRUCTURAL_LOW_FACTOR);
		assert!(probe.high_factor < probe.medium_factor);
		assert!(probe.medium_factor < probe.low_factor);
		Ok(())
	}

	#[test]
	fn structural_lod_is_baked_unit_probe() -> Result<()> {
		let built = crown(0).build();
		let a = built.structural_lod();
		let b = built.structural_lod();
		assert_eq!(a, b);
		let band = a.expect("probe");
		assert_eq!(band.high_factor, PALM_CROWN_STRUCTURAL_HIGH_FACTOR);
		assert!(band.tree_radius > 0.0);
		Ok(())
	}

	#[test]
	fn default_rachis_and_frond_counts_are_sparse() -> Result<()> {
		let shape = PalmCrownParams::default().shape;
		assert!(shape.frond_count <= 5);
		assert!(shape.spine_segments <= 2);
		Ok(())
	}

	#[test]
	fn unit_full_from_num_is_deterministic_unit_footprint() -> Result<()> {
		let a = PalmCrownParams::unit_full_from_num(7);
		let b = PalmCrownParams::unit_full_from_num(7);
		assert_eq!(a, b);
		assert_eq!(a.shape.seed, 7);
		assert_eq!(a.ring_count, 6);
		assert!((9..=11).contains(&a.shape.frond_count));
		assert!((8..=10).contains(&a.shape.spine_segments));
		assert_eq!(a.shape.leaflet_count, 16);
		assert!((a.characteristic_size() - 1.0).abs() < 1e-3);
		// Anchors are seed-independent; ring shapes / frond runs key off seed.
		assert_eq!(a.ring_shape(0).seed, 7);
		assert_ne!(a.ring_shape(0).seed, PalmCrownParams::unit_full_from_num(8).ring_shape(0).seed);
		let runs_a = a.ring_shape(0).frond_runs_at(Vec3::ZERO);
		let runs_b = PalmCrownParams::unit_full_from_num(8).ring_shape(0).frond_runs_at(Vec3::ZERO);
		assert_ne!(runs_a[0][0].direction, runs_b[0][0].direction);
		assert!(
			a.shape.droop < 0.55,
			"unit droop should shrink with characteristic size, got {}",
			a.shape.droop
		);
		Ok(())
	}

	#[test]
	fn unit_detail_from_num_is_lighter_unit_footprint() -> Result<()> {
		let a = PalmCrownParams::unit_detail_from_num(3);
		let b = PalmCrownParams::unit_detail_from_num(3);
		assert_eq!(a, b);
		assert_eq!(a.shape.seed, 3);
		assert_eq!(a.ring_count, 3);
		assert!((5..=7).contains(&a.shape.frond_count));
		assert!((4..=6).contains(&a.shape.spine_segments));
		assert!((10..=14).contains(&a.shape.leaflet_count));
		assert!((a.characteristic_size() - 1.0).abs() < 1e-3);
		// Detail is smaller / sparser than full when compared pre-normalize via counts.
		let full = PalmCrownParams::unit_full_from_num(3);
		assert!(a.shape.frond_count < full.shape.frond_count);
		assert!(a.ring_count < full.ring_count);
		assert!(a.shape.spine_segments < full.shape.spine_segments);
		Ok(())
	}

	#[test]
	fn unit_full_for_height_scales_with_stalk() -> Result<()> {
		let h = 5.0;
		let (unit, size) = PalmCrownParams::unit_full_for_height_from_num(h, 9);
		assert!((unit.characteristic_size() - 1.0).abs() < 1e-3);
		assert!(
			(size - DATE_PALM_FROND_LENGTH_FRACTION * h).abs() < 1e-3
				|| size >= DATE_PALM_FROND_LENGTH_FRACTION * h * 0.9
		);
		assert!(size > 2.0);
		Ok(())
	}

	#[test]
	fn unit_detail_for_height_scales_with_plant() -> Result<()> {
		let h = 4.0;
		let (unit, size) = PalmCrownParams::unit_detail_for_height_from_num(h, 4);
		assert!((unit.characteristic_size() - 1.0).abs() < 1e-3);
		assert!(size > 1.0);
		assert!((size - DETAIL_FROND_LENGTH_FRACTION * h).abs() < 0.5);
		Ok(())
	}

	#[test]
	fn into_unit_full_from_num_returns_world_size() -> Result<()> {
		let authored = PalmCrownParams {
			ring_count: 4,
			ring_spacing: 0.5,
			shape: FrondCrownShape {
				length: 4.0,
				width: 0.4,
				frond_count: 6,
				seed: 0,
				..PalmCrownParams::default().shape
			},
		};
		let size_before = authored.characteristic_size();
		let (unit, size) = authored.into_unit_full_from_num(11);
		assert!((size - size_before).abs() < 1e-4 || size > 0.0);
		assert_eq!(unit.shape.seed, 11);
		assert_eq!(unit.ring_count, 6);
		assert!((unit.characteristic_size() - 1.0).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn into_unit_detail_from_num_returns_world_size() -> Result<()> {
		let authored = PalmCrownParams {
			ring_spacing: 0.3,
			shape: FrondCrownShape { length: 2.5, width: 0.2, ..PalmCrownParams::default().shape },
			..PalmCrownParams::default()
		};
		let (unit, size) = authored.into_unit_detail_from_num(5);
		assert!(size > 1.0);
		assert_eq!(unit.shape.seed, 5);
		assert!((5..=7).contains(&unit.shape.frond_count));
		assert!((unit.characteristic_size() - 1.0).abs() < 1e-3);
		Ok(())
	}
}
