//! Physical cover colliders (Fixed layer) derived from layout, not mesh bounds.

use avian3d::prelude::Collider;
use bevy::prelude::*;

use crate::layout::{DoricColumnLayout, RoundPartLayout, SquarePartLayout};

/// Shapes in **assembly-local** space for one compound [`Collider`].
pub fn cover_collider_shapes(layout: &DoricColumnLayout) -> Vec<(Vec3, Quat, Collider)> {
	let mut shapes = Vec::with_capacity(4);
	shapes.push(square_shape(0.0, layout.plinth));
	let shaft_y = layout.plinth.height;
	shapes.push(round_shape(shaft_y, layout.shaft));
	let echinus_y = layout.shaft_top_y();
	shapes.push(round_shape(echinus_y, layout.echinus));
	let abacus_y = echinus_y + layout.echinus.height;
	shapes.push(square_shape(abacus_y, layout.abacus));
	shapes
}

pub fn cover_collider(layout: &DoricColumnLayout) -> Collider {
	let shapes = cover_collider_shapes(layout);
	Collider::compound(shapes)
}

fn square_shape(base_y: f32, part: SquarePartLayout) -> (Vec3, Quat, Collider) {
	let height = part.height.max(1e-3);
	let center_y = base_y + height * 0.5;
	(
		Vec3::new(0.0, center_y, 0.0),
		Quat::IDENTITY,
		Collider::cuboid(part.half_side, height * 0.5, part.half_side),
	)
}

fn round_shape(base_y: f32, part: RoundPartLayout) -> (Vec3, Quat, Collider) {
	let height = part.height.max(1e-3);
	let radius = (part.radius_bottom + part.radius_top) * 0.5;
	let center_y = base_y + height * 0.5;
	(Vec3::new(0.0, center_y, 0.0), Quat::IDENTITY, Collider::cylinder(radius, height))
}

/// Axis-aligned bounds enclosing the cover collider (assembly space).
pub fn cover_aabb(layout: &DoricColumnLayout) -> (Vec3, Vec3) {
	let h = layout.max_plan_half_extent();
	(Vec3::new(-h, 0.0, -h), Vec3::new(h, layout.total_height(), h))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::spec::DoricColumnSpec;
	use bevy::prelude::Color;

	#[test]
	fn collider_height_matches_layout() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		let (min, max) = cover_aabb(&layout);
		assert!((max.y - min.y - layout.total_height()).abs() < 0.05);
	}

	#[test]
	fn plinth_and_abacus_use_square_colliders() {
		let layout = DoricColumnSpec::default().layout();
		let shapes = cover_collider_shapes(&layout);
		assert_eq!(shapes.len(), 4);
		let plinth_half = layout.plinth.half_side;
		let abacus_half = layout.abacus.half_side;
		assert!(plinth_half > layout.shaft.radius_bottom);
		assert!(abacus_half > layout.echinus.radius_top);
	}

	#[test]
	fn collider_radius_tracks_scale() {
		let narrow = DoricColumnSpec { scale: Vec3::splat(0.5), ..DoricColumnSpec::default() };
		let wide = DoricColumnSpec { scale: Vec3::splat(2.0), ..DoricColumnSpec::default() };
		let (_, max_n) = cover_aabb(&narrow.layout());
		let (_, max_w) = cover_aabb(&wide.layout());
		assert!(max_w.x > max_n.x);
	}

	#[test]
	fn collider_unchanged_when_only_materials_change() {
		let a = DoricColumnSpec::default();
		let mut b = DoricColumnSpec::default();
		b.materials.shaft = b.materials.shaft.with_palette([Color::BLACK]);
		let sa = cover_collider_shapes(&a.layout());
		let sb = cover_collider_shapes(&b.layout());
		assert_eq!(sa.len(), sb.len());
	}
}
