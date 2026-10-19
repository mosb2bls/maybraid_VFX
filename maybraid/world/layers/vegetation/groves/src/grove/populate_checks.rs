//! Shared grove populate checks. Per-recipe tests supply definition, noise,
//! extent, and terrain; determinism and nonempty live here. Same pattern as
//! `woody_checks` for render nesting.

use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

use super::{
	ForestGroveBiases, Grove, GroveCellVariant, GroveDefinition, GroveExtent, GroveWorldSample,
};

impl<V> Grove<V>
where
	V: Clone + PartialEq + std::fmt::Debug,
{
	/// Populate twice and require identical placements.
	pub(crate) fn assert_populate_matches(
		&self,
		extent: &GroveExtent,
		world: &impl GroveWorldSample,
	) -> Result<Vec<GroveCellVariant<V>>> {
		let first = self.populate(extent, world);
		let second = self.populate(extent, world);
		anyhow::ensure!(first == second, "populate must be deterministic");
		Ok(first)
	}
}

/// Assemble with default forest biases and require deterministic populate.
pub fn assert_populated_grove_is_deterministic<V>(
	definition: GroveDefinition<V>,
	noise: NoiseParams,
	extent: GroveExtent,
	terrain: &impl GroveWorldSample,
) -> Result<Vec<GroveCellVariant<V>>>
where
	V: Clone + PartialEq + std::fmt::Debug,
{
	let grove = Grove::assemble(definition, ForestGroveBiases::default(), noise, Vec3::ZERO);
	grove.assert_populate_matches(&extent, terrain)
}

/// Same as [`assert_populated_grove_is_deterministic`], plus at least one placement.
pub fn assert_populated_grove_is_deterministic_and_non_empty<V>(
	definition: GroveDefinition<V>,
	noise: NoiseParams,
	extent: GroveExtent,
	terrain: &impl GroveWorldSample,
) -> Result<()>
where
	V: Clone + PartialEq + std::fmt::Debug,
{
	let placements = assert_populated_grove_is_deterministic(definition, noise, extent, terrain)?;
	anyhow::ensure!(!placements.is_empty(), "expected at least one placement");
	Ok(())
}
