//! [`Terrace`]: one level pad at the cell center and the confines drawn on it.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use buildings::{Confines, Openings};
use procedural_common::{NoiseParams, SeededHash};

use crate::plan::{cell_salt, inscribe_yawed_extents, sample_confines_yaw};
use crate::{Development, DevelopmentFinish, PadParams, PadPlan, SiteGround};

/// Footprint and height ranges a terrace development draws its confines from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerraceEnvelope {
	pub min_footprint: f32,
	pub max_footprint: f32,
	pub min_height: f32,
	pub max_height: f32,
	pub rotates: bool,
}

/// A development fitted to the confines of one [`Terrace`].
///
/// Every such type is a [`Development`] planned as that terrace: level at the
/// ground under the cell center and kept clear of water.
pub trait TerraceDevelopment: Sized {
	fn envelope() -> TerraceEnvelope;

	fn finish(hash: SeededHash) -> DevelopmentFinish;

	fn fit_terrace(terrace: &Terrace) -> Option<Self>;
}

/// Terrace, confines, and finish drawn for one cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Terrace {
	pub cell: Aabb3d,
	pub seed: u32,
	pub height: f32,
	pub confines_height: f32,
	pub confines_extent_xz: Vec2,
	pub confines_yaw: f32,
	pub finish: DevelopmentFinish,
}

impl Terrace {
	pub fn new<T: TerraceDevelopment>(cell: Aabb3d, height: f32, seed: u32) -> Self {
		let hash = SeededHash::new(seed.wrapping_add(cell_salt(cell)));
		let envelope = T::envelope();
		let (min_foot, max_foot) = (envelope.min_footprint, envelope.max_footprint);
		let yaw = if envelope.rotates { sample_confines_yaw(hash.unit(37)) } else { 0.0 };
		let extent_x = min_foot + (max_foot - min_foot) * hash.unit(11);
		let extent_z = min_foot + (max_foot - min_foot) * hash.unit(13);
		let confines_height =
			envelope.min_height + (envelope.max_height - envelope.min_height) * hash.unit(17);
		Self {
			cell,
			seed,
			height,
			confines_height,
			confines_extent_xz: inscribe_yawed_extents(extent_x, extent_z, yaw, max_foot),
			confines_yaw: yaw,
			finish: T::finish(hash),
		}
	}

	pub fn center_xz(&self) -> Vec2 {
		center_xz(self.cell)
	}

	/// Noise the buildings are fitted with.
	pub fn noise(&self) -> NoiseParams {
		NoiseParams { seed: self.seed as i32, ..NoiseParams::default() }
	}

	/// Unrotated confines AABB sitting on the terrace (world space).
	///
	/// Builders author against this axis-aligned box. The sampled yaw is
	/// recorded on [`Confines::roll`] and applied at host spawn about the
	/// cell center.
	pub fn confines_bounds(&self) -> Aabb3d {
		let c = self.center_xz();
		let half = self.confines_extent_xz * 0.5;
		Aabb3d::from_min_max(
			Vec3::new(c.x - half.x, self.height, c.y - half.y),
			Vec3::new(c.x + half.x, self.height + self.confines_height, c.y + half.y),
		)
	}

	/// Fitted confines: unrotated AABB plus yaw on [`Confines::roll`].
	pub fn confines(&self) -> Confines {
		Confines::new(self.confines_bounds(), self.confines_yaw, Openings::new())
	}

	/// World-axis half extents of the yawed building confines.
	pub fn footprint_half_extents(&self) -> Vec2 {
		let half = self.confines_extent_xz * 0.5;
		let (sin, cos) = self.confines_yaw.sin_cos();
		let (sin, cos) = (sin.abs(), cos.abs());
		Vec2::new(cos * half.x + sin * half.y, sin * half.x + cos * half.y)
	}

	/// The flatten and ease under the yawed confines.
	pub fn pad(&self) -> PadPlan {
		PadPlan::building_skirt(
			self.center_xz(),
			self.confines_extent_xz * 0.5,
			self.confines_yaw,
			self.height,
			PadParams::default(),
		)
	}
}

impl<T: TerraceDevelopment> Development for T {
	type Plan = Terrace;

	fn plan(
		ground: &mut impl SiteGround,
		cell: Aabb3d,
		seed: u32,
	) -> Option<(Terrace, Vec<PadPlan>)> {
		let center = center_xz(cell);
		let height = ground.height_at(center.x, center.y)?;
		let terrace = Terrace::new::<T>(cell, height, seed);
		let pad = terrace.pad();
		if ground.hydro_overlaps(&pad) {
			return None;
		}
		Some((terrace, vec![pad]))
	}

	fn build(terrace: &Terrace) -> Option<Self> {
		T::fit_terrace(terrace)
	}
}

fn center_xz(cell: Aabb3d) -> Vec2 {
	Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5)
}
