//! Wet-column query for buoyancy and other stamp consumers.
//!
//! Samples fill support at \((x, z)\). Do not raycast the presented water mesh.

use crate::water::ComposedWater;

/// Free-surface column at a horizontal sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterColumn {
	/// Free surface \(W\).
	pub surface: f32,
	/// Sibling terrain height at the same \((x, z)\).
	pub bed: f32,
}

impl WaterColumn {
	pub fn depth(self) -> f32 {
		(self.surface - self.bed).max(0.0)
	}
}

impl ComposedWater {
	/// Wet column at `(x, z)`, or `None` when every fill is dry there.
	///
	/// Union of fills uses max \(W\), matching [`Self::sign_uniform_on_y`].
	pub fn column_at(&self, x: f32, z: f32) -> Option<WaterColumn> {
		let bed = self.terrain.height_at_with_all_modulations(x, z);
		let mut wet_top = f32::NEG_INFINITY;
		let mut any_wet = false;
		for fill in &self.fills {
			if let Some((_lo, hi)) = fill.wet_y_span_at(x, z, bed) {
				any_wet = true;
				wet_top = wet_top.max(hi);
			}
		}
		if any_wet && wet_top.is_finite() {
			Some(WaterColumn { surface: wet_top, bed })
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::sdf::TerrainSdf;
	use bevy::prelude::*;
	use terrain_stamps::{CircleRegion, Region2D};
	use terrain_watersheds::{WaterFill, WaterSurface};

	fn flat_fill(level: f32, radius: f32) -> WaterFill {
		WaterFill {
			surface: WaterSurface::Flat {
				level,
				region: Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius }),
			},
		}
	}

	fn composed(fills: Vec<WaterFill>) -> ComposedWater {
		ComposedWater::compose(TerrainSdf::new(1, 20.0), fills)
	}

	#[test]
	fn dry_column_outside_the_carve() -> anyhow::Result<()> {
		let water = composed(vec![flat_fill(40.0, 10.0)]);
		assert!(water.column_at(80.0, 0.0).is_none());
		Ok(())
	}

	#[test]
	fn wet_column_matches_fill_surface_and_terrain_bed() -> anyhow::Result<()> {
		let water = composed(vec![flat_fill(40.0, 50.0)]);
		let column = water.column_at(0.0, 0.0).ok_or_else(|| anyhow::anyhow!("expected wet"))?;
		assert_eq!(column.surface, 40.0);
		assert_eq!(column.bed, water.terrain.height_at_with_all_modulations(0.0, 0.0));
		Ok(())
	}

	#[test]
	fn union_takes_the_higher_surface() -> anyhow::Result<()> {
		let water = composed(vec![flat_fill(36.0, 50.0), flat_fill(42.0, 50.0)]);
		let column = water.column_at(0.0, 0.0).ok_or_else(|| anyhow::anyhow!("expected wet"))?;
		assert_eq!(column.surface, 42.0);
		Ok(())
	}

	#[test]
	fn snapshot_matches_fill_on_the_stored_cell() -> anyhow::Result<()> {
		use crate::terrain::cell::{cell_bounds, TerrainCellLayout};
		use crate::terrain::index::{insert_water_for_test, TerrainStorage};
		use crate::water::Water;
		use lod::hcsg::HcsgStorage;

		let layout = TerrainCellLayout::default();
		let cell = cell_bounds(0, 0, layout.cell_size, layout.vertical_half_extent);
		let terrain = TerrainSdf::new(1, 20.0);
		let fills = vec![flat_fill(40.0, layout.cell_size * 2.0)];
		let sdf = ComposedWater::compose(terrain.clone(), fills.clone());
		let water = Water {
			cell,
			terrain: terrain.clone(),
			fills,
			sdf,
			material: Handle::default(),
			res_2: 4,
			stream_ring: None,
		};
		let expected = water.column_at(8.0, 8.0);
		let mut store = HcsgStorage::default();
		insert_water_for_test(&mut store, water);
		assert_eq!(store.water_column_at(&layout, 8.0, 8.0), expected);
		assert_eq!(store.water_snapshot().column(&layout, 8.0, 8.0), expected);
		Ok(())
	}

	#[test]
	fn graded_surface_tracks_the_sample() -> anyhow::Result<()> {
		let west = WaterFill {
			surface: WaterSurface::Flat {
				level: 30.0,
				region: Region2D::Circle(CircleRegion {
					center: Vec2::new(-8.0, 0.0),
					radius: 6.0,
				}),
			},
		};
		let east = WaterFill {
			surface: WaterSurface::Flat {
				level: 34.0,
				region: Region2D::Circle(CircleRegion { center: Vec2::new(8.0, 0.0), radius: 6.0 }),
			},
		};
		let water = composed(vec![west, east]);
		assert_eq!(water.column_at(-8.0, 0.0).map(|c| c.surface), Some(30.0));
		assert_eq!(water.column_at(8.0, 0.0).map(|c| c.surface), Some(34.0));
		Ok(())
	}
}
