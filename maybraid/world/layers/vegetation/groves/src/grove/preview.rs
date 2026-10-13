//! Shared preview / build surface for well-known grove params.
//!
//! Every grove CLI already copied frontend, extent, terrain, variant cap, and
//! resolved placements. Grove-specific structs flatten this type and keep only
//! flags `build` still reads (`merge_collections`, palette-seed noise).

use bevy_math::Vec2;
use gimme_gen::Cell;

use crate::grove::extent::{GroveExtent, DEFAULT_GROVE_EXTENT_XZ};
use crate::grove::frontend::GroveFrontend;
use crate::grove::terrain::{FlatTerrainSample, GroveWorldSample};
use crate::grove::{GroveCellVariant, GroveDefinition};

/// Shared authoring fields for a grove preview / isolation build.
///
/// Forest attachment later is `Params::default().with_extent(e).with_tree_variants(n).build_on(&world)`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(rename_all = "kebab-case"))]
pub struct GrovePreviewParams<V: Clone> {
	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Grove"))]
	pub grove: GroveFrontend,

	#[cfg_attr(feature = "render", arg(skip))]
	pub extent: GroveExtent,

	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Terrain"))]
	pub terrain: FlatTerrainSample,

	/// Number of unit archetypes (`unit_from_num(0..n)`). Caps unique merged-mesh
	/// handles for High/Medium. Authored / playground default is 100; assembled
	/// world forests apply a separate, lower cap. Tuft groves that also cap
	/// patches keep `patch_variants` on the grove-specific params.
	#[cfg_attr(feature = "render", arg(long, default_value_t = 100))]
	pub tree_variants: u32,

	#[cfg_attr(feature = "render", arg(skip))]
	resolved_placements: Option<Vec<GroveCellVariant<V>>>,
}

impl<V: Clone> Default for GrovePreviewParams<V> {
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			extent: GroveExtent::new(
				bevy_math::Vec3::ZERO,
				bevy_math::Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain: FlatTerrainSample::default(),
			tree_variants: 100,
			resolved_placements: None,
		}
	}
}

impl<V: Clone> GrovePreviewParams<V> {
	pub fn with_extent(mut self, extent: GroveExtent) -> Self {
		self.extent = extent;
		self
	}

	pub fn with_terrain(mut self, terrain: FlatTerrainSample) -> Self {
		self.terrain = terrain;
		self
	}

	pub fn with_tree_variants(mut self, tree_variants: u32) -> Self {
		self.tree_variants = tree_variants.max(1);
		self
	}

	pub fn with_resolved_placements(mut self, placements: Vec<GroveCellVariant<V>>) -> Self {
		self.resolved_placements = Some(placements);
		self
	}

	pub fn cell_extent_xz(&self, authored: GroveDefinition<V>) -> Vec2 {
		self.grove.definition(authored).cell_extent_xz
	}

	pub fn placement_cells(&self, authored: GroveDefinition<V>) -> Vec<Cell> {
		self.extent.subdivide_xz(self.cell_extent_xz(authored))
	}

	pub fn placements(&self, authored: GroveDefinition<V>) -> Vec<GroveCellVariant<V>> {
		self.placements_on(authored, &self.terrain)
	}

	/// Select placements against `world` ([`GroveWorldSample::height_at`]).
	pub fn placements_on(
		&self,
		authored: GroveDefinition<V>,
		world: &impl GroveWorldSample,
	) -> Vec<GroveCellVariant<V>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(authored).populate(&self.extent, world)
	}
}

/// Deref + `with_extent` / `with_tree_variants` / `placements_on` on a grove `*Params` that flattens
/// [`GrovePreviewParams`]. `$definition` is the authored `fn definition()`.
#[macro_export]
macro_rules! impl_grove_preview_params {
	($Params:ident, $Cell:ty) => {
		impl std::ops::Deref for $Params {
			type Target = $crate::grove::GrovePreviewParams<$Cell>;

			fn deref(&self) -> &Self::Target {
				&self.preview
			}
		}

		impl std::ops::DerefMut for $Params {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.preview
			}
		}

		impl $Params {
			pub fn with_extent(mut self, extent: $crate::GroveExtent) -> Self {
				self.preview.extent = extent;
				self
			}

			pub fn with_terrain(mut self, terrain: $crate::FlatTerrainSample) -> Self {
				self.preview.terrain = terrain;
				self
			}

			pub fn with_tree_variants(mut self, tree_variants: u32) -> Self {
				self.preview.tree_variants = tree_variants.max(1);
				self
			}

			pub fn cell_extent_xz(&self) -> bevy_math::Vec2 {
				self.preview.cell_extent_xz(definition())
			}

			pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
				self.preview.placement_cells(definition())
			}

			pub fn placements(&self) -> Vec<$crate::grove::GroveCellVariant<$Cell>> {
				self.preview.placements(definition())
			}

			pub fn placements_on(
				&self,
				world: &impl $crate::GroveWorldSample,
			) -> Vec<$crate::grove::GroveCellVariant<$Cell>> {
				self.preview.placements_on(definition(), world)
			}
		}
	};
}
