//! Union-first hydro primitives + broadphase helpers.
//!
//! Rim / apron bands live on [`crate::primitive::parameters::HydroParams`].
//! Terrain blend (class-priority carve / rim / apron) lives on
//! [`crate::primitive::node::HydroNode`]; complexes gather intersecting nodes.

pub mod elevation;
pub mod footprint;
pub mod index;

pub use elevation::{HydroElevation, RadialBowl, ReachProfile};
pub use footprint::{Ellipse, HydroFootprint, ReachSegment};
pub use index::FootprintIndex;

use bevy_math::{FloatExt, Vec2};

/// Soft-min / soft-max length scale for surface and elevation blends (world units).
pub const SURFACE_SMOOTHMIN_K: f32 = 1.5;

/// One hydraulic node: footprint + local elevation field.
#[derive(Debug, Clone)]
pub struct HydroPrimitive {
	pub footprint: HydroFootprint,
	pub elevation: HydroElevation,
	/// Extra AABB pad for broadphase / apron support (world units).
	pub influence_pad: f32,
}

impl HydroPrimitive {
	pub fn aabb(&self) -> (Vec2, Vec2) {
		self.footprint.aabb()
	}

	pub fn phi(&self, p: Vec2) -> f32 {
		self.footprint.sdf(p)
	}

	/// Local-frame surface and bed at `p` (valid even slightly outside; caller masks).
	pub fn surface_and_bed(&self, p: Vec2) -> (f32, f32) {
		match (&self.footprint, &self.elevation) {
			(HydroFootprint::Reach(seg), HydroElevation::Reach(profile)) => {
				let (z, x_signed) = seg.frame(p);
				let w = profile.surface_a + (profile.surface_b - profile.surface_a) * z;
				let xn = (x_signed.abs() / seg.half_width.max(1e-3)).clamp(0.0, 1.0);
				let depth = profile.center_depth.max(0.0) * transverse_bowl(xn);
				(w, w - depth)
			}
			(HydroFootprint::Ellipse(e), HydroElevation::Radial(bowl)) => {
				let u = e.radial_norm(p).clamp(0.0, 1.0);
				let depth = bowl.center_depth.max(0.0) * transverse_bowl(u);
				(bowl.surface, bowl.surface - depth)
			}
			// Mismatched footprint/elevation: fall back to flat mid values.
			(_, HydroElevation::Reach(profile)) => {
				let w = 0.5 * (profile.surface_a + profile.surface_b);
				(w, w - profile.center_depth.max(0.0))
			}
			(_, HydroElevation::Radial(bowl)) => {
				(bowl.surface, bowl.surface - bowl.center_depth.max(0.0))
			}
		}
	}
}

/// Decompose a graded corridor into per-segment reach primitives.
pub fn primitives_from_polyline(
	path: &[Vec2],
	levels: &[f32],
	half_width: f32,
	center_depth: f32,
	influence_pad: f32,
) -> Vec<HydroPrimitive> {
	let n = path.len().min(levels.len());
	if n < 2 {
		return Vec::new();
	}
	let hw = half_width.max(1e-3);
	let depth = center_depth.max(0.25);
	let mut out = Vec::with_capacity(n - 1);
	for i in 0..n - 1 {
		let a = path[i];
		let b = path[i + 1];
		if a.distance(b) <= 1e-4 {
			continue;
		}
		out.push(HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment { a, b, half_width: hw }),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: levels[i],
				surface_b: levels[i + 1],
				center_depth: depth,
			}),
			influence_pad: influence_pad.max(0.0),
		});
	}
	out
}

fn transverse_bowl(t: f32) -> f32 {
	// Cosine lobe: 1 at center, 0 at bank.
	let t = t.clamp(0.0, 1.0);
	0.5 * (1.0 + (std::f32::consts::PI * t).cos())
}

/// Polynomial smooth minimum over a list (associative fold).
pub(crate) fn smoothmin_fold(values: &[f32], k: f32) -> f32 {
	if values.is_empty() {
		return 0.0;
	}
	let k = k.max(1e-3);
	let mut acc = values[0];
	for &v in &values[1..] {
		acc = smoothmin2(acc, v, k);
	}
	acc
}

/// Polynomial smooth maximum over a list (dual of [`smoothmin_fold`]).
pub(crate) fn smoothmax_fold(values: &[f32], k: f32) -> f32 {
	if values.is_empty() {
		return 0.0;
	}
	let k = k.max(1e-3);
	let mut acc = values[0];
	for &v in &values[1..] {
		acc = -smoothmin2(-acc, -v, k);
	}
	acc
}

fn smoothmin2(a: f32, b: f32, k: f32) -> f32 {
	// Exact ties must preserve the value (polynomial softmin otherwise dips by k/4).
	if (a - b).abs() <= 1e-5 {
		return a.min(b);
	}
	let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
	b.lerp(a, h) - k * h * (1.0 - h)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::complex::HydroComplex;
	use crate::primitive::parameters::ComplexParams;
	use procedural_common::Bounds2;
	use terrain_stamps::RegionNoise;

	#[test]
	fn reach_profile_bowls_in_x_pitches_in_z() -> anyhow::Result<()> {
		let prim = HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment {
				a: Vec2::new(0.0, 0.0),
				b: Vec2::new(40.0, 0.0),
				half_width: 8.0,
			}),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: 50.0,
				surface_b: 40.0,
				center_depth: 4.0,
			}),
			influence_pad: 2.0,
		};
		let (w_mid, bed_mid) = prim.surface_and_bed(Vec2::new(20.0, 0.0));
		assert!((w_mid - 45.0).abs() < 1e-3);
		assert!((bed_mid - (45.0 - 4.0)).abs() < 1e-3);
		let (w_bank, bed_bank) = prim.surface_and_bed(Vec2::new(20.0, 8.0));
		assert!((w_bank - 45.0).abs() < 1e-3, "W independent of X");
		assert!(bed_bank > bed_mid + 2.0, "bed rises toward bank: mid={bed_mid} bank={bed_bank}");
		Ok(())
	}

	#[test]
	fn union_bed_takes_minimum() -> anyhow::Result<()> {
		let a = HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment {
				a: Vec2::new(0.0, 0.0),
				b: Vec2::new(40.0, 0.0),
				half_width: 6.0,
			}),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: 50.0,
				surface_b: 50.0,
				center_depth: 2.0,
			}),
			influence_pad: 1.0,
		};
		let b = HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment {
				a: Vec2::new(0.0, 2.0),
				b: Vec2::new(40.0, 2.0),
				half_width: 6.0,
			}),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: 50.0,
				surface_b: 50.0,
				center_depth: 8.0,
			}),
			influence_pad: 1.0,
		};
		let prep = HydroComplex::from_primitives(
			Bounds2::from_xz(-10.0, -20.0, 50.0, 30.0),
			1,
			vec![a, b],
			ComplexParams::default(),
		);
		let h = prep.modify_elevation(50.0, 20.0, 1.0);
		assert!(h <= 50.0 - 7.0, "carve soft-min should prefer deeper channel: {h}");
		Ok(())
	}

	#[test]
	fn no_internal_rim_in_confluence_interior() -> anyhow::Result<()> {
		let a = HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment {
				a: Vec2::new(0.0, 0.0),
				b: Vec2::new(40.0, 0.0),
				half_width: 8.0,
			}),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: 30.0,
				surface_b: 30.0,
				center_depth: 3.0,
			}),
			influence_pad: 1.0,
		};
		let b = HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment {
				a: Vec2::new(20.0, -20.0),
				b: Vec2::new(20.0, 20.0),
				half_width: 8.0,
			}),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: 30.0,
				surface_b: 30.0,
				center_depth: 3.0,
			}),
			influence_pad: 1.0,
		};
		let mut apron = ComplexParams::default();
		apron.rim_lift = 2.0;
		apron.rim_width = 3.0;
		apron.apron_width = 6.0;
		apron.rim_height = RegionNoise::from_seed(1, 0.05, 0.0);
		let prep = HydroComplex::from_primitives(
			Bounds2::from_xz(-30.0, -40.0, 60.0, 40.0),
			3,
			vec![a, b],
			apron,
		);
		// Junction interior should be below surface (carved), not raised.
		let h0 = 28.0;
		let h1 = prep.modify_elevation(h0, 20.0, 0.0);
		assert!(h1 <= h0 + 0.05, "confluence interior must not raise: {h0} -> {h1}");
		assert!(h1 < 30.0 - 1.0, "should sit in the carved bowl: {h1}");
		Ok(())
	}
}
