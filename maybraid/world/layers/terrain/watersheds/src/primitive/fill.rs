//! Stamp-owned water fill for Watershed pocket bodies (lakes and streams).
//!
//! Hydro fills delegate to [`HydroComplex::water_distance`]: outside the carve,
//! approximate distance to \(\phi = 0\); inside, half-space below the blended free
//! surface \(W\) (`y - W`). Flat fills (unit tests) carry an explicit region for XZ.

use crate::primitive::complex::HydroComplex;
use bevy_math::{Vec2, Vec3};
use std::sync::Arc;
use terrain_stamps::Region2D;

/// Water surface elevation model decided by the stamp.
#[derive(Debug, Clone)]
pub enum WaterSurface {
	/// Constant lake (or pool) surface — tests / simple stamps.
	Flat {
		level: f32,
		/// Horizontal support for unit-test flats (hydro uses carve \(\phi\)).
		region: Region2D,
	},
	/// Shared indexed hydrology complex (owns \(W\) blend + water SDF).
	Hydro { complex: Arc<HydroComplex> },
}

impl WaterSurface {
	/// Surface elevation at a horizontal sample (`0` when dry / undefined).
	pub fn level_at(&self, x: f32, z: f32) -> f32 {
		match self {
			Self::Flat { level, .. } => *level,
			Self::Hydro { complex } => complex.surface_at(x, z).unwrap_or(0.0),
		}
	}
}

/// Stamp-owned water volume (Lake / Stream).
///
/// Hydro: [`Self::distance`] delegates to [`HydroComplex::water_distance`].
/// Flat: region exterior ∪ half-space below \(W\).
#[derive(Debug, Clone)]
pub struct WaterFill {
	/// Water surface elevation model decided by the stamp.
	pub surface: WaterSurface,
}

impl WaterFill {
	pub fn from_hydro(complex: Arc<HydroComplex>) -> Self {
		Self { surface: WaterSurface::Hydro { complex } }
	}

	/// Representative horizontal samples for wet-volume gating.
	pub fn wet_volume_probe_points(&self) -> Vec<Vec2> {
		match &self.surface {
			WaterSurface::Hydro { complex } => {
				let mut pts: Vec<Vec2> = complex
					.hydrology
					.iter()
					.map(|node| complex.bounds.project(node.sample_point()))
					.collect();
				if pts.is_empty() {
					pts.push(complex.bounds.center());
				}
				pts
			}
			WaterSurface::Flat { region, .. } => vec![region.sample_point()],
		}
	}

	/// Surface elevation \(W\) at `(x, z)`.
	pub fn surface_level_at(&self, x: f32, z: f32) -> f32 {
		self.surface.level_at(x, z)
	}

	/// True when the XZ sample is inside the hydro carve (or Flat region).
	pub fn inside_horizontal(&self, x: f32, z: f32) -> bool {
		match &self.surface {
			WaterSurface::Hydro { complex } => complex.inside_carve(x, z),
			WaterSurface::Flat { region, .. } => region.sdf(Vec2::new(x, z)) <= 0.0,
		}
	}

	/// Vertical wet interval when the column is wet: \((-\infty, W]\).
	pub fn wet_y_span_at(&self, x: f32, z: f32, _terrain_height: f32) -> Option<(f32, f32)> {
		if !self.inside_horizontal(x, z) {
			return None;
		}
		Some((f32::NEG_INFINITY, self.surface_level_at(x, z)))
	}

	/// Water SDF at `p` given composed terrain height `terrain_height`.
	pub fn distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		match &self.surface {
			WaterSurface::Hydro { complex } => complex.water_distance(p, terrain_height),
			WaterSurface::Flat { level, region } => {
				let d_xz = region.sdf(Vec2::new(p.x, p.z));
				if d_xz > 0.0 {
					d_xz
				} else {
					p.y - *level
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::hydro::{
		Ellipse, HydroElevation, HydroFootprint, HydroPrimitive, RadialBowl,
	};
	use crate::primitive::node::HydroNode;
	use crate::primitive::parameters::HydroParams;
	use terrain_stamps::CircleRegion;

	#[test]
	fn flat_half_space_below_w_inside_region() -> anyhow::Result<()> {
		let w = 40.0;
		let fill = WaterFill {
			surface: WaterSurface::Flat {
				level: w,
				region: Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 50.0 }),
			},
		};
		let h = 36.0;
		let span = fill.wet_y_span_at(0.0, 0.0, h).expect("wet");
		assert!(span.0.is_infinite() && span.0.is_sign_negative());
		assert_eq!(span.1, w);
		assert!(fill.distance(Vec3::new(0.0, w - 1.0, 0.0), h) < 0.0);
		assert!(fill.distance(Vec3::new(0.0, w + 1.0, 0.0), h) > 0.0);
		// Half-space continues below the bed (terrain occludes in the scene).
		assert!(fill.distance(Vec3::new(0.0, h - 1.0, 0.0), h) < 0.0);
		// Outside region → positive XZ distance.
		assert!(fill.distance(Vec3::new(80.0, 38.0, 0.0), h) > 0.0);
		Ok(())
	}

	#[test]
	fn hydro_probe_points_are_clamped_to_complex_bounds() {
		let bounds = procedural_common::Bounds2::from_xz(0.0, 0.0, 160.0, 160.0);
		let node = HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse(Ellipse {
					center: Vec2::new(200.0, 80.0),
					radii: Vec2::splat(60.0),
					rotation: 0.0,
				}),
				elevation: HydroElevation::Radial(RadialBowl { surface: 40.0, center_depth: 3.0 }),
				influence_pad: 12.0,
			},
			HydroParams::default(),
			12.0,
		);
		let fill =
			WaterFill::from_hydro(Arc::new(HydroComplex::new(bounds, 1).with_hydro(vec![node])));

		assert_eq!(fill.wet_volume_probe_points(), vec![Vec2::new(160.0, 80.0)]);
		assert!(fill.inside_horizontal(160.0, 80.0));
	}
}
