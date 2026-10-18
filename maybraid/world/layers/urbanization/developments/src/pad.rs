//! Building pads as a layout plans them: where ground is levelled or graded.
//!
//! A [`PadPlan`] only describes the earthwork. Whoever places the development
//! on real ground turns it into a terrain modulation.

use bevy_math::Vec2;

use crate::{BuildingFootprint, PlacedBuilding};

/// Skirt ease (metres) from the flatten berm out to identity terrain.
pub const PAD_EDGE_EASE: f32 = 10.0;

/// Extra flatten (metres) outside the building footprint so walls sit on the pad, not the ease.
pub const PAD_BERM: f32 = 4.0;

/// Rounded-rect corner radius (metres) on the flatten footprint.
pub const PAD_ROUND: f32 = 2.0;

/// Ease / berm parameters for one pad form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadParams {
	/// Extra flatten outside the building footprint (metres).
	pub berm: f32,
	/// Ease / apron width from flatten support out to identity terrain.
	pub ease: f32,
	/// Rounded-rect corner radius on the flatten footprint.
	pub round: f32,
}

impl Default for PadParams {
	fn default() -> Self {
		Self { berm: PAD_BERM, ease: PAD_EDGE_EASE, round: PAD_ROUND }
	}
}

impl PadParams {
	/// Connecting grade: core wide enough to hit a ~5 m terrain sample pitch,
	/// short ease so overlapping skirts do not fill the whole cell.
	pub fn path() -> Self {
		Self { berm: 2.0, ease: 16.0, round: 0.0 }
	}

	/// Shepherds house / hut terrace. Berm covers a sample pitch past the walls
	/// so the floor sits on flatten, not the interpolated ease.
	pub fn shepherds() -> Self {
		Self { berm: 6.0, ease: 16.0, round: PAD_ROUND }
	}

	/// Shared market terrace: compact apron around a whole stall cluster.
	pub fn market() -> Self {
		Self { berm: 3.0, ease: 12.0, round: 2.0 }
	}

	/// Flatten half-extents: building plan plus berm.
	pub fn flatten_half(self, building_half: Vec2) -> Vec2 {
		building_half + Vec2::splat(self.berm.max(0.0))
	}

	/// Full modulation half-extents: flatten support plus the ease skirt.
	pub fn influence_half(self, building_half: Vec2) -> Vec2 {
		self.flatten_half(building_half) + Vec2::splat(self.ease.max(0.0))
	}
}

/// One levelled or graded piece of ground.
#[derive(Debug, Clone, PartialEq)]
pub enum PadForm {
	/// Level terrace under a yawed building rectangle, widened by the berm.
	Flatten { center: Vec2, building_half_extents: Vec2, yaw: f32, height: f32, params: PadParams },
	/// Ground graded from `height_a` at `a` to `height_b` at `b`.
	Grade { a: Vec2, b: Vec2, half_width: f32, height_a: f32, height_b: f32, params: PadParams },
}

/// The pad one part of a development stands on.
#[derive(Debug, Clone, PartialEq)]
pub struct PadPlan {
	/// Representative level: the terrace floor, or a lane's mean.
	pub height: f32,
	pub forms: Vec<PadForm>,
}

impl PadPlan {
	/// One rectangular flatten terrace for a yawed building plan.
	pub fn building_skirt(
		center: Vec2,
		building_half_extents: Vec2,
		yaw: f32,
		height: f32,
		params: PadParams,
	) -> Self {
		Self {
			height,
			forms: vec![PadForm::Flatten { center, building_half_extents, yaw, height, params }],
		}
	}

	/// One grade per non-degenerate polyline segment, at the mean of the end levels.
	pub fn graded_polyline(
		path: &[Vec2],
		levels: &[f32],
		half_width: f32,
		params: PadParams,
	) -> Self {
		let height = levels.first().zip(levels.last()).map(|(a, b)| 0.5 * (a + b)).unwrap_or(0.0);
		let n = path.len().min(levels.len());
		let half_width = half_width.max(1e-3);
		let forms = (0..n.saturating_sub(1))
			.filter(|&i| path[i].distance(path[i + 1]) > 1e-4)
			.map(|i| PadForm::Grade {
				a: path[i],
				b: path[i + 1],
				half_width,
				height_a: levels[i],
				height_b: levels[i + 1],
				params,
			})
			.collect();
		Self { height, forms }
	}

	pub fn is_empty(&self) -> bool {
		self.forms.is_empty()
	}
}

impl<T: BuildingFootprint> PlacedBuilding<T> {
	/// One flatten per authored footprint piece, posed by the same center and
	/// yaw used to present the building.
	pub fn pad_plan(&self, params: PadParams) -> PadPlan {
		let center = self.center_xz;
		let yaw = self.yaw;
		let (sin, cos) = yaw.sin_cos();
		let forms = self
			.building
			.footprint_rects()
			.into_iter()
			.map(|rect| {
				let local = (rect.min + rect.max) * 0.5 - center;
				PadForm::Flatten {
					center: center
						+ Vec2::new(cos * local.x + sin * local.y, -sin * local.x + cos * local.y),
					building_half_extents: (rect.max - rect.min) * 0.5,
					yaw,
					height: self.ground_height,
					params,
				}
			})
			.collect();
		PadPlan { height: self.ground_height, forms }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn graded_polyline_skips_degenerate_segments() {
		let path = [Vec2::ZERO, Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0)];
		let plan = PadPlan::graded_polyline(&path, &[4.0, 4.0, 6.0, 8.0], 2.0, PadParams::path());
		assert_eq!(plan.forms.len(), 2);
		assert!((plan.height - 6.0).abs() < 1e-6);
	}
}
