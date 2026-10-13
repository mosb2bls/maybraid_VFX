//! [`RichmondGround`]: the ground developments are generated over.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use durham::{Durham, Terrain};
use lod::hcsg::{self, GenerationContext};
use procedural_common::Bounds2;
use terrain_layer_model::OnTerrain;
use terrain_watersheds::{WaterFill, WaterSurface};
use urbanization_developments::{PadPlan, SiteGround};

use crate::compose::PadComposable;
use crate::pad::PadComplex;
use crate::padded::TerrainWithPads;

/// A ground model whose cells developments sample and pads compose into.
///
/// Its cells are generated in [`HcsgStorage`] like any other node, so a
/// development reads them as generation dependencies.
pub trait RichmondGround: Send + Sync + 'static {
	type Cell: GroundCell + PadComposable<Padded = TerrainWithPads> + hcsg::GenerationScheme;
}

impl RichmondGround for OnTerrain<Durham> {
	type Cell = Terrain;
}

/// One ground cell's composed surface, as developments sample it.
pub trait GroundCell {
	fn bounds(&self) -> Aabb3d;

	/// Composed height (every ground modulation applied) at `(x, z)`.
	fn composed_height_at(&self, x: f32, z: f32) -> f32;

	/// Whether the cell's water overlaps `bounds`.
	fn hydro_overlaps(&self, bounds: Bounds2) -> bool;
}

impl GroundCell for Terrain {
	fn bounds(&self) -> Aabb3d {
		self.cell
	}

	fn composed_height_at(&self, x: f32, z: f32) -> f32 {
		self.sdf.terrain().height_at_with_all_modulations(x, z)
	}

	fn hydro_overlaps(&self, bounds: Bounds2) -> bool {
		hydro_overlaps_xz(&self.marazion_fills, bounds)
	}
}

/// True when `bounds` overlaps any hydro primitive support in `fills`.
pub fn hydro_overlaps_xz(fills: &[WaterFill], bounds: Bounds2) -> bool {
	for fill in fills {
		match &fill.surface {
			WaterSurface::Hydro { complex } => {
				for node in &complex.hydrology {
					if node.correction_intersects(bounds) {
						return true;
					}
				}
			}
			WaterSurface::Flat { region, .. } => {
				let c = bounds.center();
				if region.sdf(c) <= 0.0 {
					return true;
				}
				let corners = [
					bounds.min,
					Vec2::new(bounds.max.x, bounds.min.y),
					bounds.max,
					Vec2::new(bounds.min.x, bounds.max.y),
				];
				if corners.iter().any(|p| region.sdf(*p) <= 0.0) {
					return true;
				}
			}
		}
	}
	false
}

/// [`SiteGround`] over the cells of `G` under one site, finest first.
///
/// The cells are the ones `G` originates in the site, generated through the
/// context, so a site samples the same ground whatever else is stored.
pub struct GroundCells<G: RichmondGround> {
	cells: Vec<Arc<G::Cell>>,
}

impl<G: RichmondGround> GroundCells<G> {
	pub fn generate(cx: &mut GenerationContext, site: Aabb3d) -> Self {
		let cells = cx.get_or_generate_in_finest_first::<G::Cell>(site, |cell| cell.bounds());
		Self { cells }
	}
}

/// Pads are checked over their realized support, flatten plus ease.
impl<G: RichmondGround> SiteGround for GroundCells<G> {
	fn height_at(&mut self, x: f32, z: f32) -> Option<f32> {
		self.cells
			.iter()
			.find(|cell| {
				let bounds = cell.bounds();
				x >= bounds.min.x && x <= bounds.max.x && z >= bounds.min.z && z <= bounds.max.z
			})
			.map(|cell| cell.composed_height_at(x, z))
	}

	fn hydro_overlaps(&mut self, pad: &PadPlan) -> bool {
		let pad = PadComplex::from(pad).bounds;
		self.cells.iter().any(|cell| {
			let bounds = cell.bounds();
			pad.min.x <= bounds.max.x
				&& pad.max.x >= bounds.min.x
				&& pad.min.y <= bounds.max.z
				&& pad.max.y >= bounds.min.z
				&& cell.hydro_overlaps(pad)
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use terrain_stamps::{CircleRegion, Region2D};

	#[test]
	fn flat_fill_overlaps_center() {
		let fills = vec![WaterFill {
			surface: WaterSurface::Flat {
				level: 10.0,
				region: Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 20.0 }),
			},
		}];
		let hit = Bounds2::from_xz(-5.0, -5.0, 5.0, 5.0);
		let miss = Bounds2::from_xz(80.0, 80.0, 90.0, 90.0);
		assert!(hydro_overlaps_xz(&fills, hit));
		assert!(!hydro_overlaps_xz(&fills, miss));
	}
}
