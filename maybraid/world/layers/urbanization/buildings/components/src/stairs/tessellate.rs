//! Private stair kit tessellation (not part of the public IR).

use std::f32::consts::TAU;

use bevy_math::Vec3;

use crate::placed::{Placed, Placement};
use crate::stairs::geometry::{SpiralStair, StairGeometry, StraightStair};

/// Walkable kit half-extent (\(X = Y = Z \in [-1, 1]\)).
pub(crate) const TREAD_HALF_EXTENT: f32 = 1.0;
/// Extra kit units in \(-X\) (authored support; mesh \(X \to -2\)).
pub(crate) const TREAD_BLEED_X: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Spiral / Straight reserved for non-tread placeholders
pub(crate) enum StairKit {
	/// Single rough-stone tread cube (`rough_stonework_tread_001`).
	Tread,
	Spiral,
	Straight,
}

impl StairGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<StairKit>> {
		match self {
			Self::Spiral(g) => spiral_kits(g),
			Self::Straight(g) => straight_kits(g),
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<StairKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}

fn spiral_kits(g: &SpiralStair) -> Vec<Placed<StairKit>> {
	let tops = g.effective_tread_tops();
	if tops.is_empty() {
		return Vec::new();
	}
	let n = tops.len() as f32;
	let yaw_step = g.turns * TAU / n;
	let half = 2.0 * TREAD_HALF_EXTENT;
	let mut prev_top = 0.0_f32;

	tops.into_iter()
		.enumerate()
		.map(|(i, top)| {
			let rise = (top - prev_top).max(1e-4);
			prev_top = top;
			let yaw = i as f32 * yaw_step;
			let (s, c) = yaw.sin_cos();
			// Centerline on the circle; yaw so local +X is tangential (ascent).
			// Kit: left face = −Z; bleed support extends toward −X.
			let translation = Vec3::new(c * g.radius, top - 0.5 * rise, -s * g.radius);
			let scale = Vec3::new(g.tread_depth / half, rise / half, g.tread_width / half);
			Placed::new(StairKit::Tread, translation, yaw + std::f32::consts::FRAC_PI_2)
				.with_scale(scale)
		})
		.collect()
}

fn straight_kits(g: &StraightStair) -> Vec<Placed<StairKit>> {
	let tops = g.effective_tread_tops();
	if tops.is_empty() {
		return Vec::new();
	}
	let going = g.going_per_tread();
	let half = 2.0 * TREAD_HALF_EXTENT;
	let mut prev_top = 0.0_f32;

	tops.into_iter()
		.enumerate()
		.map(|(i, top)| {
			let rise = (top - prev_top).max(1e-4);
			prev_top = top;
			let (x, scale_x) = kit_x(g.flush_start && i == 0, i, going);
			let scale = Vec3::new(scale_x, rise / half, g.width / half);
			Placed::new(StairKit::Tread, Vec3::new(x, top - 0.5 * rise, 0.0), 0.0).with_scale(scale)
		})
		.collect()
}

/// Walkable cube maps going onto \([-1, 1]\). Flush packs \(X \to -2\) into that going.
fn kit_x(flush_first: bool, i: usize, going: f32) -> (f32, f32) {
	let center_x = i as f32 * going;
	let half = 2.0 * TREAD_HALF_EXTENT;
	if flush_first {
		let kit_min_x = -(TREAD_HALF_EXTENT + TREAD_BLEED_X);
		let scale_x = going / (TREAD_HALF_EXTENT - kit_min_x);
		(center_x - 0.5 * going - kit_min_x * scale_x, scale_x)
	} else {
		(center_x, going / half)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn kit_world_x(placement_x: f32, scale_x: f32, kit_x: f32) -> f32 {
		placement_x + kit_x * scale_x
	}

	fn two_tread(flush: bool) -> Vec<Placed<StairKit>> {
		let g = StraightStair::run(0.36, 0.5, 0.5, 0.25)
			.with_tread_tops(vec![0.18, 0.36])
			.with_flush_start(flush);
		StairGeometry::Straight(g).kit_pieces()
	}

	#[test]
	fn flush_start_packs_bleed_into_the_first_going() {
		let going = 0.25;
		let nested = two_tread(false);
		let flush = two_tread(true);
		assert_eq!(nested.len(), 2);
		assert_eq!(flush.len(), 2);

		let nest = &nested[0].placement;
		assert!(nest.translation.x.abs() < 1e-4);
		assert!((nest.scale.x - going / 2.0).abs() < 1e-4);
		let nest_bleed =
			kit_world_x(nest.translation.x, nest.scale.x, -(TREAD_HALF_EXTENT + TREAD_BLEED_X));
		assert!(
			(nest_bleed + going).abs() < 1e-4,
			"nested bleed hangs a half going behind, got {nest_bleed}"
		);

		let first = &flush[0].placement;
		let mesh_trail =
			kit_world_x(first.translation.x, first.scale.x, -(TREAD_HALF_EXTENT + TREAD_BLEED_X));
		let mesh_lead = kit_world_x(first.translation.x, first.scale.x, TREAD_HALF_EXTENT);
		assert!(
			(mesh_trail + 0.5 * going).abs() < 1e-4,
			"flush mesh trailing should be the walkable trailing, got {mesh_trail}"
		);
		assert!(
			(mesh_lead - 0.5 * going).abs() < 1e-4,
			"flush mesh leading should stay on the walkable leading, got {mesh_lead}"
		);

		assert!((flush[1].placement.translation.x - going).abs() < 1e-4);
		assert!((flush[1].placement.scale.x - going / 2.0).abs() < 1e-4);
	}
}
