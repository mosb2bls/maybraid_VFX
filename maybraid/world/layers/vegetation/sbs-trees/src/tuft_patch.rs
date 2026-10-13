//! **Tuft Patch** — a few blade tufts scattered over a small ground area.
//!
//! Unlike the single-anchor tufts, which radiate every blade from one point, a tuft patch
//! deterministically picks a few anchor points within an XZ footprint and grows a blade tuft
//! at each, reading as one loose clump of grass rather than a fountain.
//!
//! [`TuftPatchParams::build`] grows clump anchors once into [`TuftPatch`], which implements
//! [`VegetationComponents`] via one [`FoliageNode`] / [`FrondCollection`] for the whole patch
//! (all clump blades merged — one LOD probe). Use [`TuftPatch::merge`] /
//! [`TuftPatch::merge_placed`] to fold many patches into fewer collections when probe count
//! matters (e.g. grove authorship).

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use sbs_geometry::BladeTuftShape;
use vegetation_components::{
	FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode, VegetationComponents,
};

/// Authoring / CLI parameters for a tuft patch.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct TuftPatchParams {
	/// Number of tuft clumps scattered over the patch.
	#[arg(long, default_value_t = 5)]
	pub clump_count: u32,

	/// Square patch footprint side length (m) the clumps scatter within.
	#[arg(long, default_value_t = 1.5)]
	pub patch_extent_xz: f32,

	#[command(flatten, next_help_heading = "Blade Tuft")]
	pub shape: BladeTuftShape,
}

impl Default for TuftPatchParams {
	fn default() -> Self {
		Self { clump_count: 5, patch_extent_xz: 1.5, shape: BladeTuftShape::default() }
	}
}

impl TuftPatchParams {
	pub fn new(clump_count: u32, patch_extent_xz: f32, shape: BladeTuftShape) -> Self {
		Self { clump_count, patch_extent_xz, shape }
	}

	/// Unit-footprint patch (`patch_extent_xz = 1`) whose layout noise is driven solely by `num`.
	///
	/// Use with a world [`Placement`] scale so many plants share one archetypal mesh.
	pub fn unit_from_num(num: u32) -> Self {
		Self::unit_from_num_shaped(num, BladeTuftShape::default())
	}

	/// Like [`Self::unit_from_num`], but keeps authored shape metrics (length / width / …).
	///
	/// Overwrites `shape.seed` with `num` and forces `patch_extent_xz = 1`. Clump count stays
	/// at the default unless the caller sets it after.
	pub fn unit_from_num_shaped(num: u32, mut shape: BladeTuftShape) -> Self {
		shape.seed = num as i32;
		Self { clump_count: Self::default().clump_count, patch_extent_xz: 1.0, shape }
	}

	/// Normalize this params set to unit footprint keyed by `num`.
	///
	/// Returns `(unit_params, world_size)` where `world_size` is the pre-normalize scale to
	/// apply on the plant [`Placement`] (`max(patch_extent_xz, blade_length)`).
	pub fn into_unit_from_num(mut self, num: u32) -> (Self, f32) {
		let size = self.patch_extent_xz.max(self.shape.blade_length).max(1e-4);
		let inv = 1.0 / size;
		self.shape.blade_length *= inv;
		self.shape.blade_width *= inv;
		self.shape.base_spread *= inv;
		self.patch_extent_xz = if self.patch_extent_xz > 1e-3 { 1.0 } else { 0.0 };
		self.shape.seed = num as i32;
		(self, size)
	}

	/// Deterministic patch-local clump anchors, scattered within the XZ footprint.
	pub fn clump_anchors(&self) -> Vec<Vec3> {
		let config =
			NoiseConfig::new(NoiseParams::from_scalar(self.shape.seed as f32, 1.0, 1.0, 1));
		let half = (self.patch_extent_xz * 0.5).max(0.0);
		(0..self.clump_count)
			.map(|i| {
				// Non-integer lanes keep samples off gradient-noise lattice points.
				let lane = (i as f32 + 0.5) * 3.7;
				let x = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 1.0);
				let z = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 2.0);
				Vec3::new(x, 0.0, z)
			})
			.collect()
	}

	/// The authored shape re-seeded for clump `index`, so clumps differ in blade layout.
	pub fn clump_shape(&self, index: u32) -> BladeTuftShape {
		BladeTuftShape {
			seed: self.shape.seed.wrapping_add((index as i32 + 1) * 131),
			..self.shape.clone()
		}
	}

	/// Grow clump anchors once for presentation / LOD emission.
	pub fn build(&self) -> TuftPatch {
		TuftPatch::from_params(self)
	}
}

/// Built tuft patch: params, resolved clump anchors, and baked frond runs (one LOD collection).
#[derive(Clone, Debug, PartialEq)]
pub struct TuftPatch {
	pub clump_count: u32,
	pub patch_extent_xz: f32,
	pub shape: BladeTuftShape,
	pub anchors: Vec<Vec3>,
	/// Patch-local frond runs (all clumps). Source of truth for emission / merge.
	runs: Vec<FrondRun>,
}

impl TuftPatch {
	pub fn from_params(params: &TuftPatchParams) -> Self {
		let anchors = params.clump_anchors();
		let mut runs = Vec::new();
		for (index, anchor) in anchors.iter().enumerate() {
			runs.extend(Self::clump_runs_from(&params.clump_shape(index as u32), *anchor));
		}
		Self {
			clump_count: params.clump_count,
			patch_extent_xz: params.patch_extent_xz,
			shape: params.shape.clone(),
			anchors,
			runs,
		}
	}

	/// Unit-footprint patch from [`TuftPatchParams::unit_from_num`].
	pub fn unit_from_num(num: u32) -> Self {
		TuftPatchParams::unit_from_num(num).build()
	}

	/// Unit-footprint patch from [`TuftPatchParams::unit_from_num_shaped`].
	pub fn unit_from_num_shaped(num: u32, shape: BladeTuftShape) -> Self {
		TuftPatchParams::unit_from_num_shaped(num, shape).build()
	}

	fn clump_runs_from(shape: &BladeTuftShape, anchor: Vec3) -> Vec<FrondRun> {
		// One FrondRun per blade; chained segments keep kink connectivity under merge LOD.
		shape
			.frond_runs_at(anchor)
			.into_iter()
			.filter_map(|run| {
				let placements: Vec<Placement> = run
					.into_iter()
					.filter_map(|seg| {
						Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
					})
					.collect();
				(!placements.is_empty()).then(|| FrondRun::from_placements(placements))
			})
			.collect()
	}

	/// Patch-local frond runs (all clumps).
	pub fn frond_runs(&self) -> &[FrondRun] {
		&self.runs
	}

	/// Bake `placement` into every frond segment (compose as parent of each member).
	pub fn apply_placement(&mut self, placement: Placement) {
		if placement == Placement::IDENTITY {
			return;
		}
		for run in &mut self.runs {
			for member in &mut run.segments {
				member.placement = placement.compose_child(member.placement);
			}
		}
		for anchor in &mut self.anchors {
			*anchor = placement.compose_child(Placement::new(*anchor, 0.0)).translation;
		}
	}

	/// Append another patch's frond runs (same local frame — bake placements first).
	pub fn merge(&mut self, other: TuftPatch) {
		self.clump_count = self.clump_count.saturating_add(other.clump_count);
		self.patch_extent_xz = self.patch_extent_xz.max(other.patch_extent_xz);
		self.anchors.extend(other.anchors);
		self.runs.extend(other.runs);
	}

	/// Fold placed patches into at most `target_count` patches (spatially sorted chunks).
	///
	/// Each input placement is baked into that patch's runs before merging. Result patches use
	/// identity placement (geometry already in the shared parent frame).
	///
	/// `target_count == 0` means **no fold**: one output patch per input, with placement baked in.
	pub fn merge_placed(
		patches: impl IntoIterator<Item = (Placement, TuftPatch)>,
		target_count: usize,
	) -> Vec<TuftPatch> {
		let mut remaining: Vec<(Placement, TuftPatch)> = patches.into_iter().collect();
		if remaining.is_empty() {
			return Vec::new();
		}
		if target_count == 0 {
			return remaining
				.into_iter()
				.map(|(placement, mut patch)| {
					patch.apply_placement(placement);
					patch
				})
				.collect();
		}
		remaining.sort_by(|a, b| {
			a.0.translation
				.x
				.total_cmp(&b.0.translation.x)
				.then(a.0.translation.z.total_cmp(&b.0.translation.z))
		});
		let chunk_len = remaining.len().div_ceil(target_count);
		let mut out = Vec::with_capacity(target_count.min(remaining.len()));
		while !remaining.is_empty() {
			let take = chunk_len.min(remaining.len());
			let chunk: Vec<(Placement, TuftPatch)> = remaining.drain(..take).collect();
			let mut iter = chunk.into_iter();
			let (placement, mut merged) = iter.next().expect("chunk non-empty");
			merged.apply_placement(placement);
			for (placement, mut next) in iter {
				next.apply_placement(placement);
				merged.merge(next);
			}
			out.push(merged);
		}
		out
	}

	/// Cheap LOD probe from anchors + footprint / blade length (no frond walk).
	fn lod_probe(&self) -> (Vec3, f32) {
		let blade = self.shape.blade_length.max(1e-4);
		let footprint = (self.patch_extent_xz * 0.5 * std::f32::consts::SQRT_2).max(blade);
		if self.anchors.is_empty() {
			return (Vec3::ZERO, footprint);
		}
		let n = self.anchors.len() as f32;
		let center = self.anchors.iter().fold(Vec3::ZERO, |acc, a| acc + *a) / n;
		let mut radius = footprint;
		for anchor in &self.anchors {
			radius = radius.max(anchor.distance(center) + blade);
		}
		(center, radius.max(1e-4))
	}

	/// One frond collection for the whole patch (one LOD probe).
	fn foliage_nodes(&self) -> Vec<FoliageNode> {
		if self.runs.is_empty() {
			return Vec::new();
		}
		let (center, radius) = self.lod_probe();
		vec![FoliageNode::frond_collection(
			FrondCollection::new(self.runs.clone()).with_probe(center, radius),
			Placement::IDENTITY,
		)]
	}
}

impl VegetationComponents for TuftPatch {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			// Collections keep an UltraLow marker themselves; structural UltraLow drops them.
			LodSceneLevel::UltraLow => Layers::new(),
			_ => Layers::from_free(self.foliage_nodes()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::scene::LodSceneLevel;

	fn patch(seed: i32) -> TuftPatchParams {
		TuftPatchParams {
			shape: BladeTuftShape { seed, ..BladeTuftShape::default() },
			..TuftPatchParams::default()
		}
	}

	#[test]
	fn anchors_stay_within_patch_footprint() -> Result<()> {
		let patch = patch(7);
		let anchors = patch.clump_anchors();
		assert_eq!(anchors.len(), patch.clump_count as usize);
		let half = patch.patch_extent_xz * 0.5;
		for anchor in &anchors {
			assert!(anchor.x.abs() <= half);
			assert!(anchor.z.abs() <= half);
			assert_eq!(anchor.y, 0.0);
		}
		Ok(())
	}

	#[test]
	fn anchors_are_deterministic_per_seed_and_scattered() -> Result<()> {
		let anchors = patch(7).clump_anchors();
		assert_eq!(anchors, patch(7).clump_anchors());
		let distinct = anchors
			.iter()
			.enumerate()
			.all(|(i, a)| anchors.iter().skip(i + 1).all(|b| a.distance(*b) > 1e-4));
		assert!(distinct, "expected scattered anchors, got {anchors:?}");
		Ok(())
	}

	#[test]
	fn clump_shapes_vary_by_seed_only() -> Result<()> {
		let patch = patch(7);
		let a = patch.clump_shape(0);
		let b = patch.clump_shape(1);
		assert_ne!(a.seed, b.seed);
		assert_eq!(a.blade_count, b.blade_count);
		assert_eq!(a.blade_length, b.blade_length);
		Ok(())
	}

	#[test]
	fn build_emits_one_collection_for_the_patch() -> Result<()> {
		let params = TuftPatchParams {
			clump_count: 2,
			shape: BladeTuftShape {
				blade_count: 4,
				bend_segments: 2,
				seed: 3,
				..BladeTuftShape::default()
			},
			..TuftPatchParams::default()
		};
		let built = params.build();
		let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1, "one collection / LOD probe per patch");
		let collection = nodes[0].geometry.as_frond_collection().expect("collection geom");
		// 2 clumps × 4 blades (runs), each with 2 bend segments.
		assert_eq!(collection.runs.len(), 8);
		assert_eq!(collection.runs[0].segments.len(), 2);
		let medium = collection.runs_for_level(LodSceneLevel::Medium);
		assert_eq!(medium.len(), 4);
		assert_eq!(medium[0].segments.len(), 2, "Medium keeps full kink chains");
		let ultra = collection.runs_for_level(LodSceneLevel::UltraLow);
		assert_eq!(ultra.len(), 1);
		assert_eq!(ultra[0].segments.len(), 1, "UltraLow collapses to one chord");
		Ok(())
	}

	#[test]
	fn merge_concatenates_runs() -> Result<()> {
		let a = patch(1).build();
		let b = patch(2).build();
		let runs_a = a.frond_runs().len();
		let runs_b = b.frond_runs().len();
		let mut merged = a;
		merged.merge(b);
		assert_eq!(merged.frond_runs().len(), runs_a + runs_b);
		assert_eq!(merged.clump_count, TuftPatchParams::default().clump_count * 2);
		Ok(())
	}

	#[test]
	fn merge_placed_caps_patch_count_and_bakes_translation() -> Result<()> {
		let patches = (0..10).map(|i| {
			let placement = Placement::new(Vec3::new(i as f32 * 10.0, 0.0, 0.0), 0.0);
			let patch = TuftPatchParams {
				clump_count: 1,
				patch_extent_xz: 0.0,
				shape: BladeTuftShape {
					blade_count: 2,
					bend_segments: 1,
					seed: i,
					..BladeTuftShape::default()
				},
			}
			.build();
			(placement, patch)
		});
		let merged = TuftPatch::merge_placed(patches, 3);
		assert_eq!(merged.len(), 3);
		let total_runs: usize = merged.iter().map(|p| p.frond_runs().len()).sum();
		assert_eq!(total_runs, 20, "10 patches × 2 blades");
		// First chunk owns x=0..30; a blade base should sit near a placement translation.
		let first_base = merged[0].frond_runs()[0].segments[0].placement.translation;
		assert!(
			first_base.x.abs() < 1.0 || (first_base.x - 10.0).abs() < 1.0,
			"expected baked world X near a placement, got {first_base:?}"
		);
		Ok(())
	}

	#[test]
	fn merge_placed_zero_keeps_one_patch_per_input() -> Result<()> {
		let patches = (0..4).map(|i| {
			(
				Placement::new(Vec3::new(i as f32 * 5.0, 0.0, 0.0), 0.0),
				TuftPatchParams {
					clump_count: 1,
					patch_extent_xz: 0.0,
					shape: BladeTuftShape {
						blade_count: 1,
						bend_segments: 1,
						seed: i,
						..BladeTuftShape::default()
					},
				}
				.build(),
			)
		});
		let out = TuftPatch::merge_placed(patches, 0);
		assert_eq!(out.len(), 4);
		assert!((out[2].frond_runs()[0].segments[0].placement.translation.x - 10.0).abs() < 1.0);
		Ok(())
	}

	#[test]
	fn unit_from_num_is_unit_footprint_and_deterministic() -> Result<()> {
		let a = TuftPatch::unit_from_num(7);
		let b = TuftPatch::unit_from_num(7);
		assert_eq!(a.patch_extent_xz, 1.0);
		assert_eq!(a.shape.seed, 7);
		assert_eq!(a.anchors, b.anchors);
		assert_ne!(a.anchors, TuftPatch::unit_from_num(8).anchors);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let params = TuftPatchParams {
			clump_count: 3,
			patch_extent_xz: 4.0,
			shape: BladeTuftShape { blade_length: 2.0, seed: 0, ..BladeTuftShape::default() },
		};
		let (unit, size) = params.into_unit_from_num(3);
		assert!((size - 4.0).abs() < 1e-5);
		assert_eq!(unit.patch_extent_xz, 1.0);
		assert_eq!(unit.shape.seed, 3);
		assert!((unit.shape.blade_length - 0.5).abs() < 1e-5);
		Ok(())
	}
}
