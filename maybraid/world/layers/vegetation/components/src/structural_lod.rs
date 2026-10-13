//! Tree-level structural LOD banding (distance / tree-radius factors).
//!
//! Plain data returned from [`crate::VegetationComponents::structural_lod`] — not an
//! ECS component. Host presentation uses [`crate::FlattenedComponentsOnly`] (plants)
//! or a grove type that implements [`lod::scene::LodScene`].

use bevy::prelude::Transform;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::scene::{LodSceneLevel, LodSceneStatus};

use crate::lod_band::DistanceLodBand;

/// High when `distance / tree_radius ≤` this. Trees that call [`StructuralLod::new`]
/// without [`StructuralLod::with_factors`] inherit these; most trees author their own.
pub const STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
/// Medium when at or below this multiple of tree radius.
pub const STRUCTURAL_MEDIUM_FACTOR: f32 = 20.0;
/// Low when at or below this multiple of tree radius.
pub const STRUCTURAL_LOW_FACTOR: f32 = 32.0;

/// Viewer distance band for whole-tree structural thinning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructuralLod {
	pub center: Vec3,
	/// Characteristic footprint / canopy radius used as the distance unit.
	pub tree_radius: f32,
	pub high_factor: f32,
	pub medium_factor: f32,
	pub low_factor: f32,
	/// When true, band UltraLow maps to [`LodSceneLevel::UltraLow`].
	pub preserve_ultra_low: bool,
}

impl Default for StructuralLod {
	fn default() -> Self {
		Self {
			center: Vec3::ZERO,
			tree_radius: 1.0,
			high_factor: STRUCTURAL_HIGH_FACTOR,
			medium_factor: STRUCTURAL_MEDIUM_FACTOR,
			low_factor: STRUCTURAL_LOW_FACTOR,
			preserve_ultra_low: false,
		}
	}
}

impl StructuralLod {
	pub fn new(center: Vec3, tree_radius: f32) -> Self {
		Self {
			center,
			tree_radius: tree_radius.max(1e-4),
			high_factor: STRUCTURAL_HIGH_FACTOR,
			medium_factor: STRUCTURAL_MEDIUM_FACTOR,
			low_factor: STRUCTURAL_LOW_FACTOR,
			preserve_ultra_low: false,
		}
	}

	/// Distance unit: max(horizontal footprint, half height) so a tall narrow tree
	/// stays High while it still fills the view.
	pub fn characteristic_radius(footprint_radius: f32, height: f32) -> f32 {
		footprint_radius.max(height * 0.5).max(1e-4)
	}

	/// [`Self::new`] with [`Self::characteristic_radius`]. Chain [`Self::with_factors`]
	/// at the tree to set High / Medium / Low edges.
	pub fn from_extent(center: Vec3, footprint_radius: f32, height: f32) -> Self {
		Self::new(center, Self::characteristic_radius(footprint_radius, height))
	}

	pub fn with_factors(mut self, high: f32, medium: f32, low: f32) -> Self {
		self.high_factor = high;
		self.medium_factor = medium;
		self.low_factor = low;
		self
	}

	pub fn with_preserve_ultra_low(mut self, preserve: bool) -> Self {
		self.preserve_ultra_low = preserve;
		self
	}

	fn factor_for(self, viewer: &Transform) -> f32 {
		viewer.translation.distance(self.center) / self.tree_radius.max(1e-4)
	}

	fn band_to_level(self, band: DistanceLodBand) -> LodSceneLevel {
		match band {
			DistanceLodBand::High => LodSceneLevel::High,
			DistanceLodBand::Medium => LodSceneLevel::Medium,
			DistanceLodBand::Low => LodSceneLevel::Low,
			DistanceLodBand::UltraLow if self.preserve_ultra_low => LodSceneLevel::UltraLow,
			DistanceLodBand::UltraLow => LodSceneLevel::Low,
		}
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		self.band_to_level(DistanceLodBand::from_factors(
			self.factor_for(viewer),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		))
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.band_to_level(DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center)
				/ self.tree_radius.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		let curr = self.band_to_level(DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center)
				/ self.tree_radius.max(1e-4),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	/// Local AABB covering the structural footprint (for indexing volumes).
	pub fn footprint_aabb(self) -> Aabb3d {
		let r = self.tree_radius.max(1.0);
		let half = Vec3::new(r, r.max(2.0), r);
		Aabb3d::from_min_max(self.center - half, self.center + half)
	}
}
