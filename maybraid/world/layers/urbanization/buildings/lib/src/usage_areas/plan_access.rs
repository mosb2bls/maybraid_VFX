//! Parameterizable access / room metrics for plan packing.
//!
//! Typologies (residential, commercial, …) pass a [`PlanAccessParams`] rather
//! than hard-coding parallel thresholds for walk clear, door face, suite join,
//! and room min extent.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;
use procedural_common::aabb2_area;

const EPS: f32 = 1e-3;

/// Applied to authored door / passage leaf width and height.
pub const DOOR_SIZE_SCALE: f32 = 1.25;
/// Nominal authored door / passage face width (m) — residential default.
pub const DOOR_WIDTH: f32 = 1.0 * DOOR_SIZE_SCALE;
/// Soft max for synthetic / edge door faces (m).
pub const DOOR_WIDTH_MAX: f32 = 1.15 * DOOR_SIZE_SCALE;
/// Nominal authored door leaf height (m).
pub const DOOR_HEIGHT: f32 = 2.1 * DOOR_SIZE_SCALE;
/// Floor when a storey is shorter than [`DOOR_HEIGHT`].
pub const DOOR_HEIGHT_MIN: f32 = 1.9 * DOOR_SIZE_SCALE;
/// Soft ceiling used when a storey is taller than a door.
pub const DOOR_HEIGHT_SOFT_MAX: f32 = 2.15 * DOOR_SIZE_SCALE;

/// Leaf height that fits `storey_height` without punching the header.
pub fn door_leaf_height(storey_height: f32) -> f32 {
	storey_height.min(DOOR_HEIGHT_SOFT_MAX).max(DOOR_HEIGHT_MIN.min(storey_height))
}
/// Minimum usable room edge (m) — residential default for plan scraps / RLA.
pub const MIN_ROOM: f32 = 2.2;
/// Default suite-join shared-edge length (m).
pub const MIN_GROUP_CONNECTIVITY: f32 = 2.0;
/// Default clear width for apartment walkways / RLA spines (m).
pub const DEFAULT_WALK_CLEAR: f32 = 1.5;
/// Soft aspect ceiling when growing near a catalog target.
pub const DEFAULT_SOFT_ASPECT: f32 = 3.25;

/// Knobs for classifying plan rects and joining cells into suites.
///
/// Construct via [`PlanAccessParams::residential`] (or [`Default`]) and override
/// fields / builders for other typologies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanAccessParams {
	/// Minimum edge (m) for a usable room rectangle.
	pub room_min: f32,
	/// Corridor / open-hall clear width (m).
	pub walk_clear: f32,
	/// Authored door-face width (m). Hall clear may exceed this.
	pub door_clear: f32,
	/// Minimum shared-edge length (m) for suite grow / absorb / merge.
	pub group_connect: f32,
	/// Aspect above which near-target grow refuses snakes.
	pub soft_aspect: f32,
	/// Minimum footprint area (m²) for a body room rect (0 ⇒ derive from `room_min`).
	pub min_room_area: f32,
}

impl Default for PlanAccessParams {
	fn default() -> Self {
		Self::residential()
	}
}

impl PlanAccessParams {
	/// Defaults used by I-apartment / livable packing.
	pub fn residential() -> Self {
		Self {
			room_min: MIN_ROOM,
			walk_clear: DEFAULT_WALK_CLEAR,
			door_clear: DOOR_WIDTH,
			group_connect: MIN_GROUP_CONNECTIVITY,
			soft_aspect: DEFAULT_SOFT_ASPECT,
			min_room_area: 4.0,
		}
	}

	pub fn with_room_min(mut self, room_min: f32) -> Self {
		self.room_min = room_min.max(EPS);
		self
	}

	pub fn with_walk_clear(mut self, walk_clear: f32) -> Self {
		self.walk_clear = walk_clear.max(EPS);
		self
	}

	pub fn with_door_clear(mut self, door_clear: f32) -> Self {
		self.door_clear = door_clear.max(EPS);
		self
	}

	pub fn with_group_connect(mut self, group_connect: f32) -> Self {
		self.group_connect = group_connect.max(EPS);
		self
	}

	pub fn with_soft_aspect(mut self, soft_aspect: f32) -> Self {
		self.soft_aspect = soft_aspect.max(1.0);
		self
	}

	pub fn with_min_room_area(mut self, min_room_area: f32) -> Self {
		self.min_room_area = min_room_area.max(0.0);
		self
	}

	pub fn room_min_vec(self) -> Vec2 {
		Vec2::splat(self.room_min.max(EPS))
	}

	/// Door-face contact required on open circulation (never demands wider than
	/// the authored door when hall clear is larger).
	pub fn door_contact(self) -> f32 {
		self.door_clear.min(self.walk_clear).max(0.7)
	}

	/// Soft shared-edge length for open↔open / entry tip connectivity.
	pub fn open_touch(self) -> f32 {
		(self.walk_clear * 0.5).max(EPS)
	}

	pub fn room_area_floor(self) -> f32 {
		if self.min_room_area > EPS {
			self.min_room_area
		} else {
			self.room_min * self.room_min
		}
	}

	/// Both edges ≥ `room_min` and area above the room floor.
	pub fn is_room_rect(self, r: Aabb2d) -> bool {
		let s = r.max - r.min;
		s.x + EPS >= self.room_min
			&& s.y + EPS >= self.room_min
			&& aabb2_area(r) > self.room_area_floor()
	}

	/// Long-thin access stem: walkable but below room min extent.
	pub fn is_access_corridor(self, r: Aabb2d) -> bool {
		let s = (r.max - r.min).max(Vec2::splat(EPS));
		let min_e = s.x.min(s.y);
		let max_e = s.x.max(s.y);
		min_e + EPS >= self.walk_clear
			&& min_e + EPS < self.room_min
			&& max_e + EPS >= self.room_min
	}

	/// Both edges at least walk clear (corridor cell or better).
	pub fn is_walkable(self, r: Aabb2d) -> bool {
		let s = r.max - r.min;
		s.x + EPS >= self.walk_clear && s.y + EPS >= self.walk_clear
	}

	/// High-aspect, short min-extent, or tiny-area footprint.
	pub fn is_degenerate_footprint(self, min_ext: f32, aspect: f32, area: f32) -> bool {
		aspect > self.soft_aspect
			|| min_ext + EPS < self.room_min
			|| area + EPS < self.room_min * self.room_min * 1.5
	}
}

/// Axis-aligned footprint metrics for a set of plan cells / rects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupFootprint {
	pub bounds: Aabb2d,
	pub area: f32,
	pub aspect: f32,
	pub min_ext: f32,
	pub compact: f32,
}

impl GroupFootprint {
	pub fn from_bounds(bounds: Aabb2d, area: f32) -> Self {
		let s = (bounds.max - bounds.min).max(Vec2::splat(EPS));
		let bbox_a = (s.x * s.y).max(EPS);
		Self {
			bounds,
			area,
			aspect: s.x.max(s.y) / s.x.min(s.y),
			min_ext: s.x.min(s.y),
			compact: area / bbox_a,
		}
	}

	pub fn from_rects(rects: &[Aabb2d]) -> Option<Self> {
		let first = *rects.first()?;
		let mut b = first;
		let mut area = 0.0_f32;
		for r in rects {
			b.min = b.min.min(r.min);
			b.max = b.max.max(r.max);
			area += aabb2_area(*r);
		}
		Some(Self::from_bounds(b, area))
	}

	pub fn is_degenerate(self, access: PlanAccessParams) -> bool {
		access.is_degenerate_footprint(self.min_ext, self.aspect, self.area)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn rect(min: Vec2, max: Vec2) -> Aabb2d {
		Aabb2d { min, max }
	}

	#[test]
	fn residential_door_contact_caps_at_door_clear() {
		let a = PlanAccessParams::residential().with_walk_clear(1.5);
		assert!((a.door_contact() - DOOR_WIDTH).abs() < 1e-4);
	}

	#[test]
	fn corridor_vs_room_classification() {
		let a = PlanAccessParams::residential();
		let stem = rect(Vec2::ZERO, Vec2::new(1.8, 8.0));
		let body = rect(Vec2::ZERO, Vec2::new(6.0, 5.0));
		assert!(a.is_access_corridor(stem));
		assert!(!a.is_room_rect(stem));
		assert!(a.is_room_rect(body));
		assert!(!a.is_access_corridor(body));
	}

	#[test]
	fn builders_override_fields() {
		let a = PlanAccessParams::residential().with_room_min(3.0).with_group_connect(1.5);
		assert!((a.room_min - 3.0).abs() < 1e-4);
		assert!((a.group_connect - 1.5).abs() < 1e-4);
	}
}
