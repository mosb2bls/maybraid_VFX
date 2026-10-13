//! World sampling and per-variant placement constraints ([RFC-183 3.4.1.5–6, 3.4.2.4]).

use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::{Vec3, Vec3A};
use procedural_common::UnitRange;

/// Terrain height field usable by terrain-bound grove world samples.
///
/// Heights are world metres. The default steepness is the magnitude of a
/// one-metre forward finite difference on the height field.
pub trait GroveTerrain: Send + Sync {
	/// Surface height in world metres at `position` (XZ).
	fn height_at(&self, position: Vec3) -> f32;

	fn steepness_at(&self, position: Vec3) -> f32 {
		const EPS: f32 = 1.0;
		let h = self.height_at(position);
		let hx = self.height_at(position + Vec3::new(EPS, 0.0, 0.0));
		let hz = self.height_at(position + Vec3::new(0.0, 0.0, EPS));
		let dx = (hx - h) / EPS;
		let dz = (hz - h) / EPS;
		(dx * dx + dz * dz).sqrt()
	}
}

/// Generic [`GroveWorldSample`] adapter backed by a [`GroveTerrain`].
#[derive(Debug, Clone, Copy)]
pub struct TerrainGroveSample<T> {
	pub terrain: T,
}

impl<T> TerrainGroveSample<T> {
	pub fn new(terrain: T) -> Self {
		Self { terrain }
	}

	pub fn into_inner(self) -> T {
		self.terrain
	}
}

impl<T: GroveTerrain> GroveWorldSample for TerrainGroveSample<T> {
	fn height_at(&self, position: Vec3) -> f32 {
		self.terrain.height_at(position)
	}

	fn steepness_at(&self, position: Vec3) -> f32 {
		self.terrain.steepness_at(position)
	}
}

/// World-space height, steepness, and placement exclusion at positions.
///
/// [`Self::height_at`] is world metres (plant Y). Constraint bands stay authored on
/// buckets but are not evaluated here — normalization is forest/region policy.
pub trait GroveWorldSample {
	/// Surface height in world metres at `position` (XZ).
	fn height_at(&self, position: Vec3) -> f32;
	fn steepness_at(&self, position: Vec3) -> f32;

	/// Axis-aligned regions where grove items must not be placed.
	fn exclusion_zones(&self) -> &[Aabb3d] {
		&[]
	}

	/// Whether a grove item may occupy `position` on this sample layer.
	fn allows_placement_at(&self, position: Vec3) -> bool {
		!point_in_any_aabb(position, self.exclusion_zones())
	}
}

/// Post-process a base surface height (building pads, berms, local berms, …).
///
/// Implementers fold into [`ModulatedGroveSample`] so forests and groves can sit
/// on development-modulated ground without baking a specific terrain crate into
/// selection.
pub trait GroveHeightModulation: Send + Sync {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32;
}

impl<F> GroveHeightModulation for F
where
	F: Fn(f32, f32, f32) -> f32 + Send + Sync,
{
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		(self)(base_height, x, z)
	}
}

/// Zero or more [`GroveHeightModulation`] layers applied after a base sample.
pub trait GroveHeightModulationStack: Send + Sync {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32;
}

impl GroveHeightModulationStack for () {
	fn modulate_height(&self, base_height: f32, _x: f32, _z: f32) -> f32 {
		base_height
	}
}

impl<M: GroveHeightModulation + ?Sized> GroveHeightModulationStack for &M {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		GroveHeightModulation::modulate_height(*self, base_height, x, z)
	}
}

impl<M: GroveHeightModulation> GroveHeightModulationStack for [M] {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		self.iter().fold(base_height, |height, layer| {
			GroveHeightModulation::modulate_height(layer, height, x, z)
		})
	}
}

impl<M: GroveHeightModulation, const N: usize> GroveHeightModulationStack for [M; N] {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		self.as_slice().modulate_height(base_height, x, z)
	}
}

impl<M: GroveHeightModulation> GroveHeightModulationStack for Vec<M> {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		self.as_slice().modulate_height(base_height, x, z)
	}
}

impl<M: GroveHeightModulation + ?Sized> GroveHeightModulationStack for Option<&M> {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		match self {
			Some(layer) => GroveHeightModulation::modulate_height(*layer, base_height, x, z),
			None => base_height,
		}
	}
}

/// [`GroveWorldSample`] that applies a modulation stack on top of a base field.
#[derive(Debug, Clone, Copy)]
pub struct ModulatedGroveSample<Base, Mods = ()> {
	pub base: Base,
	pub modulations: Mods,
}

impl<Base, Mods> ModulatedGroveSample<Base, Mods> {
	pub fn new(base: Base, modulations: Mods) -> Self {
		Self { base, modulations }
	}

	pub fn plain(base: Base) -> ModulatedGroveSample<Base, ()> {
		ModulatedGroveSample { base, modulations: () }
	}
}

impl<Base, Mods> GroveWorldSample for ModulatedGroveSample<Base, Mods>
where
	Base: GroveWorldSample,
	Mods: GroveHeightModulationStack,
{
	fn height_at(&self, position: Vec3) -> f32 {
		let base = self.base.height_at(position);
		self.modulations.modulate_height(base, position.x, position.z)
	}

	fn steepness_at(&self, position: Vec3) -> f32 {
		// Finite-difference on the modulated surface so pads affect slope gates.
		const EPS: f32 = 1.0;
		let h = self.height_at(position);
		let hx = self.height_at(position + Vec3::new(EPS, 0.0, 0.0));
		let hz = self.height_at(position + Vec3::new(0.0, 0.0, EPS));
		let dx = (hx - h) / EPS;
		let dz = (hz - h) / EPS;
		(dx * dx + dz * dz).sqrt()
	}

	fn exclusion_zones(&self) -> &[Aabb3d] {
		self.base.exclusion_zones()
	}

	fn allows_placement_at(&self, position: Vec3) -> bool {
		self.base.allows_placement_at(position)
	}
}

/// Uniform world sample for CLI previews and isolation tests.
///
/// `elevation` is a constant world-metre height (CLI flag kept for existing scripts).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Terrain"))]
pub struct FlatTerrainSample {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.0))]
	pub elevation: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.1))]
	pub steepness: f32,
}

impl Default for FlatTerrainSample {
	fn default() -> Self {
		Self { elevation: 0.0, steepness: 0.1 }
	}
}

/// World-metre height from a function (Durham adapter, tests).
pub struct FnHeightSample<F>(pub F);

impl<F: Fn(Vec3) -> f32> GroveWorldSample for FnHeightSample<F> {
	fn height_at(&self, position: Vec3) -> f32 {
		(self.0)(position)
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		0.0
	}
}

impl GroveWorldSample for FlatTerrainSample {
	fn height_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// Elevation and steepness ranges attached to each bucketed variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementConstraints {
	pub elevation: UnitRange,
	pub steepness: UnitRange,
}

impl PlacementConstraints {
	pub const fn new(elevation: UnitRange, steepness: UnitRange) -> Self {
		Self { elevation, steepness }
	}

	pub const UNCONSTRAINED: Self =
		Self { elevation: UnitRange::new(0.0, 1.0), steepness: UnitRange::new(0.0, 1.0) };

	/// Whether normalized elevation and steepness satisfy this variant's half-open ranges.
	pub fn allows(&self, elevation: f32, steepness: f32) -> bool {
		scalar_in_half_open_range(elevation, self.elevation)
			&& scalar_in_half_open_range(steepness, self.steepness)
	}
}

fn point_in_any_aabb(position: Vec3, zones: &[Aabb3d]) -> bool {
	let point = Aabb3d::from_min_max(Vec3A::from(position), Vec3A::from(position));
	zones.iter().any(|zone| zone.contains(&point))
}

fn scalar_in_half_open_range(value: f32, range: UnitRange) -> bool {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	value >= lo && value < hi
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn constraints_use_half_open_ranges() -> Result<()> {
		let constraints =
			PlacementConstraints::new(UnitRange::new(0.2, 0.6), UnitRange::new(0.0, 0.3));
		assert!(constraints.allows(0.5, 0.1));
		assert!(constraints.allows(0.2, 0.0));
		assert!(!constraints.allows(0.6, 0.1), "upper bound is exclusive");
		assert!(!constraints.allows(0.5, 0.9));
		Ok(())
	}

	#[test]
	fn flat_sample_allows_all_placements() -> Result<()> {
		let sample = FlatTerrainSample::default();
		assert!(sample.allows_placement_at(Vec3::ZERO));
		assert!(sample.allows_placement_at(Vec3::new(100.0, 0.0, -50.0)));
		Ok(())
	}

	#[test]
	fn exclusion_zones_block_placement() -> Result<()> {
		struct SampleWithExclusion {
			zones: Vec<Aabb3d>,
		}

		impl GroveWorldSample for SampleWithExclusion {
			fn height_at(&self, _position: Vec3) -> f32 {
				0.5
			}

			fn steepness_at(&self, _position: Vec3) -> f32 {
				0.1
			}

			fn exclusion_zones(&self) -> &[Aabb3d] {
				&self.zones
			}
		}

		let sample =
			SampleWithExclusion { zones: vec![Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)] };
		assert!(!sample.allows_placement_at(Vec3::new(0.5, 0.5, 0.5)));
		assert!(sample.allows_placement_at(Vec3::new(2.0, 0.0, 0.0)));
		Ok(())
	}

	#[test]
	fn modulated_sample_folds_layers_in_order() -> Result<()> {
		let base = FlatTerrainSample { elevation: 10.0, steepness: 0.0 };
		let sample = ModulatedGroveSample::new(base, [|h, _, _| h + 2.0, |h, _, _| h * 2.0]);
		assert!((sample.height_at(Vec3::ZERO) - 24.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn generic_terrain_sample_delegates_height_and_steepness() -> Result<()> {
		struct SlopedTerrain;

		impl GroveTerrain for SlopedTerrain {
			fn height_at(&self, position: Vec3) -> f32 {
				2.0 * position.x + 3.0 * position.z + 7.0
			}
		}

		let sample = TerrainGroveSample::new(SlopedTerrain);
		let position = Vec3::new(4.0, 99.0, 5.0);
		assert!((sample.height_at(position) - 30.0).abs() < 1e-5);
		assert!((sample.steepness_at(position) - 13.0_f32.sqrt()).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn optional_modulation_stack_is_a_no_op_when_absent() -> Result<()> {
		let base = FlatTerrainSample { elevation: 3.0, steepness: 0.0 };
		let absent = ModulatedGroveSample::new(base, None::<&dyn GroveHeightModulation>);
		assert!((absent.height_at(Vec3::ZERO) - 3.0).abs() < 1e-5);
		let bump = |h: f32, _: f32, _: f32| h + 1.5;
		let present = ModulatedGroveSample::new(base, Some(&bump as &dyn GroveHeightModulation));
		assert!((present.height_at(Vec3::ZERO) - 4.5).abs() < 1e-5);
		Ok(())
	}
}
