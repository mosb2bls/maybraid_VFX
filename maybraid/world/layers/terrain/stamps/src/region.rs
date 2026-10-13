//! 2D stamp footprints with optional boundary noise.

mod polyline;

pub use polyline::{closest_on_polyline, grade_along_polyline, ClosestOnPolyline, PolylineRegion};

use bevy_math::Vec2;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType, Smoothstep01};

#[derive(Debug, Clone)]
pub struct RectRegion {
	pub center: Vec2,
	pub half_extents: Vec2,
	pub round: f32,
}

#[derive(Debug, Clone)]
pub struct CircleRegion {
	pub center: Vec2,
	pub radius: f32,
}

/// Axis-aligned in a local frame: half-axes [`Self::radii`] after [`Self::rotation`].
#[derive(Debug, Clone)]
pub struct EllipseRegion {
	pub center: Vec2,
	/// Half-axes in the local frame (`x` along the rotated major/minor basis).
	pub radii: Vec2,
	/// Radians; local +x is rotated by this from world +x.
	pub rotation: f32,
}

/// 2D region types with signed distance φ(x, z).
#[derive(Debug, Clone)]
pub enum Region2D {
	Rect(RectRegion),
	Circle(CircleRegion),
	Ellipse(EllipseRegion),
	/// Stadium-chain corridor along a polyline.
	Polyline(PolylineRegion),
	/// Boolean union of child footprints (`sdf = min(children)`).
	Union(Vec<Region2D>),
}

/// Optional noise for perturbing region boundaries (wobbly footprints).
///
/// Holds a ready [`NoiseConfig`] because this type is internal to stamp
/// evaluation (not an authoring / CLI surface). Prefer constructing from
/// [`NoiseParams`] ([`Self::from_params`], [`Self::from_seed`]) when you need
/// the flexible param bundle; frequency and amplitude live there.
///
/// Set [`Self::expand_only`] so samples never shrink the geometric footprint
/// (`d += −|raw|`).
#[derive(Clone)]
pub struct RegionNoise {
	pub noise: NoiseConfig,
	/// When true, boundary samples only **expand** the region (never shrink).
	pub expand_only: bool,
}

impl std::fmt::Debug for RegionNoise {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RegionNoise")
			.field("noise_params", self.noise.params())
			.field("expand_only", &self.expand_only)
			.finish()
	}
}

impl RegionNoise {
	pub fn new(noise: NoiseConfig) -> Self {
		Self { noise, expand_only: false }
	}

	pub fn from_params(params: NoiseParams) -> Self {
		Self::new(NoiseConfig::new(params))
	}

	pub fn from_seed(seed: u32, frequency: f32, amplitude: f32) -> Self {
		Self::from_params(NoiseParams {
			seed: seed as i32,
			frequency,
			amplitude,
			octaves: 1,
			noise_type: NoiseType::Perlin,
		})
	}

	/// Like [`Self::from_seed`], but boundary noise only expands the footprint.
	pub fn from_seed_expand_only(seed: u32, frequency: f32, amplitude: f32) -> Self {
		Self { expand_only: true, ..Self::from_seed(seed, frequency, amplitude) }
	}

	pub fn expand_only(mut self) -> Self {
		self.expand_only = true;
		self
	}

	pub fn sample_boundary(&self, p: Vec2) -> f32 {
		let raw = self.noise.sample_2d(p);
		if self.expand_only {
			// `Region2D` does `d += sample`; negative sample expands.
			-raw.abs()
		} else {
			raw
		}
	}

	/// Raw height-domain sample (bipolar, amplitude from noise params).
	pub fn sample_height(&self, p: Vec2) -> f32 {
		self.noise.sample_2d(p)
	}
}

impl EllipseRegion {
	/// World-space point → local frame scaled by inverse radii (`length` = 1 on shore).
	pub fn unit_local(&self, p: Vec2) -> Vec2 {
		let d = p - self.center;
		let (s, c) = self.rotation.sin_cos();
		let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
		let rx = self.radii.x.max(1e-3);
		let rz = self.radii.y.max(1e-3);
		Vec2::new(local.x / rx, local.y / rz)
	}

	/// Approximate analytic SDF (Inigo Quilez `k0*(k0-1)/k1` form).
	pub fn sdf(&self, p: Vec2) -> f32 {
		let d = p - self.center;
		let (s, c) = self.rotation.sin_cos();
		let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
		let ab = self.radii.max(Vec2::splat(1e-3));
		let k0 = (local / ab).length();
		if k0 < 1e-8 {
			return -ab.min_element();
		}
		let k1 = (local / (ab * ab)).length();
		k0 * (k0 - 1.0) / k1.max(1e-6)
	}
}

impl Region2D {
	/// Convenience constructor for a boolean union of footprints.
	pub fn union(children: Vec<Region2D>) -> Self {
		Self::Union(children)
	}

	pub fn center(&self) -> Vec2 {
		match self {
			Self::Rect(r) => r.center,
			Self::Circle(c) => c.center,
			Self::Ellipse(e) => e.center,
			Self::Polyline(p) => p.sample_point(),
			Self::Union(children) => children.first().map(|c| c.center()).unwrap_or(Vec2::ZERO),
		}
	}

	/// Representative interior sample point (same as [`Self::center`] for most shapes).
	pub fn sample_point(&self) -> Vec2 {
		self.center()
	}

	/// Normalized radial coordinate: `0` at center, `1` on the geometric shore.
	pub fn radial_norm(&self, p: Vec2) -> f32 {
		match self {
			Self::Circle(CircleRegion { center, radius }) => {
				(p - *center).length() / radius.max(1e-3)
			}
			Self::Ellipse(e) => e.unit_local(p).length(),
			Self::Rect(RectRegion { center, half_extents, .. }) => {
				let q = (p - *center).abs() / half_extents.max(Vec2::splat(1e-3));
				q.max_element()
			}
			Self::Polyline(poly) => {
				let half = poly.half_width.max(1e-3);
				(poly.sdf(p) / half + 1.0).clamp(0.0, 2.0)
			}
			Self::Union(children) => {
				children.iter().map(|c| c.radial_norm(p)).fold(f32::INFINITY, f32::min)
			}
		}
	}

	pub fn sdf(&self, p: Vec2) -> f32 {
		self.sdf_with_noise(p, None)
	}

	pub fn sdf_with_noise(&self, p: Vec2, noise: Option<&RegionNoise>) -> f32 {
		let mut d = match self {
			Region2D::Rect(RectRegion { center, half_extents, round }) => {
				let q = (p - *center).abs() - *half_extents + Vec2::splat(*round);
				let outside = q.max(Vec2::ZERO).length() - *round;
				let inside = q.x.max(q.y).min(0.0);
				outside + inside
			}
			Region2D::Circle(CircleRegion { center, radius }) => (p - *center).length() - *radius,
			Region2D::Ellipse(e) => e.sdf(p),
			Region2D::Polyline(poly) => poly.sdf(p),
			Region2D::Union(children) => {
				if children.is_empty() {
					f32::INFINITY
				} else {
					children.iter().map(|c| c.sdf(p)).fold(f32::INFINITY, f32::min)
				}
			}
		};
		if let Some(noise_config) = noise {
			d += noise_config.sample_boundary(p);
		}
		d
	}

	/// Softmask weight in `[0, 1]`: 0 deep inside, 1 outside the outer band.
	pub fn softmask_weight(
		&self,
		p: Vec2,
		inner_radius: f32,
		outer_radius: f32,
		noise: Option<&RegionNoise>,
	) -> f32 {
		let d = self.sdf_with_noise(p, noise);
		let outer = outer_radius.max(inner_radius + 0.001);
		if d < -inner_radius {
			0.0
		} else if d > outer {
			1.0
		} else {
			let t = (d + inner_radius) / (inner_radius + outer);
			Smoothstep01::eval(t)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn expand_only_never_shrinks_circle() -> anyhow::Result<()> {
		let region = Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 10.0 });
		let noise = RegionNoise::from_seed_expand_only(7, 0.05, 2.0);
		for i in 0..32 {
			let ang = i as f32 * std::f32::consts::TAU / 32.0;
			let p = Vec2::new(ang.cos(), ang.sin()) * 10.0;
			let d0 = region.sdf(p);
			let d1 = region.sdf_with_noise(p, Some(&noise));
			assert!(d1 <= d0 + 1e-4, "expand-only must not increase SDF (shrink): d0={d0} d1={d1}");
		}
		Ok(())
	}

	#[test]
	fn ellipse_sdf_negative_inside_positive_outside() -> anyhow::Result<()> {
		let e = EllipseRegion { center: Vec2::ZERO, radii: Vec2::new(20.0, 10.0), rotation: 0.0 };
		assert!(e.sdf(Vec2::ZERO) < -1.0);
		assert!(e.sdf(Vec2::new(19.0, 0.0)) < 0.0);
		assert!(e.sdf(Vec2::new(21.0, 0.0)) > 0.0);
		assert!(e.sdf(Vec2::new(0.0, 11.0)) > 0.0);
		assert!((e.unit_local(Vec2::new(20.0, 0.0)).length() - 1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn ellipse_radial_norm_respects_axes() -> anyhow::Result<()> {
		let region = Region2D::Ellipse(EllipseRegion {
			center: Vec2::ZERO,
			radii: Vec2::new(40.0, 20.0),
			rotation: 0.0,
		});
		assert!((region.radial_norm(Vec2::new(40.0, 0.0)) - 1.0).abs() < 1e-4);
		assert!((region.radial_norm(Vec2::new(0.0, 20.0)) - 1.0).abs() < 1e-4);
		assert!(region.radial_norm(Vec2::new(20.0, 0.0)) < 0.6);
		Ok(())
	}

	#[test]
	fn union_sdf_is_min_of_children() -> anyhow::Result<()> {
		let a = Region2D::Circle(CircleRegion { center: Vec2::new(-20.0, 0.0), radius: 8.0 });
		let b = Region2D::Circle(CircleRegion { center: Vec2::new(20.0, 0.0), radius: 8.0 });
		let u = Region2D::union(vec![a.clone(), b.clone()]);
		let p_a = Vec2::new(-20.0, 0.0);
		let p_b = Vec2::new(20.0, 0.0);
		let p_mid = Vec2::ZERO;
		assert!((u.sdf(p_a) - a.sdf(p_a)).abs() < 1e-5);
		assert!((u.sdf(p_b) - b.sdf(p_b)).abs() < 1e-5);
		assert!((u.sdf(p_mid) - a.sdf(p_mid).min(b.sdf(p_mid))).abs() < 1e-5);
		assert!(u.sdf(p_a) < 0.0);
		assert!(u.sdf(p_b) < 0.0);
		assert!(u.sdf(p_mid) > 0.0);
		Ok(())
	}

	#[test]
	fn union_softmask_wet_inside_either_child() -> anyhow::Result<()> {
		let a = Region2D::Circle(CircleRegion { center: Vec2::new(-15.0, 0.0), radius: 6.0 });
		let b = Region2D::Circle(CircleRegion { center: Vec2::new(15.0, 0.0), radius: 6.0 });
		let u = Region2D::union(vec![a, b]);
		assert!(u.softmask_weight(Vec2::new(-15.0, 0.0), 0.0, 2.0, None) < 0.05);
		assert!(u.softmask_weight(Vec2::new(15.0, 0.0), 0.0, 2.0, None) < 0.05);
		assert!(u.softmask_weight(Vec2::ZERO, 0.0, 2.0, None) >= 0.99);
		Ok(())
	}
}
