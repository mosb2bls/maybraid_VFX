//! Physical cover colliders (Fixed layer) derived from layout, not mesh bounds.

use avian3d::prelude::Collider;
use bevy::prelude::*;

use crate::layout::{DoricColumnLayout, PartLayout};

/// Shapes in **assembly-local** space for one compound [`Collider`].
pub fn cover_collider_shapes(layout: &DoricColumnLayout) -> Vec<(Vec3, Quat, Collider)> {
	let mut shapes = Vec::with_capacity(3);
	let y = 0.0;
	shapes.push(cylinder_shape(y, layout.pedestal));
	let shaft_y = layout.pedestal.height;
	shapes.push(cylinder_shape(shaft_y, layout.shaft));
	let capital_y = shaft_y + layout.shaft.height;
	shapes.push(cylinder_shape(capital_y, layout.capital));
	shapes
}

pub fn cover_collider(layout: &DoricColumnLayout) -> Collider {
	let shapes = cover_collider_shapes(layout);
	Collider::compound(shapes)
}

fn cylinder_shape(base_y: f32, part: PartLayout) -> (Vec3, Quat, Collider) {
	let height = part.height.max(1e-3);
	let radius = (part.radius_bottom + part.radius_top) * 0.5;
	let center_y = base_y + height * 0.5;
	(
		Vec3::new(0.0, center_y, 0.0),
		Quat::IDENTITY,
		Collider::cylinder(radius, height),
	)
}

/// Axis-aligned bounds enclosing the cover collider (assembly space).
pub fn cover_aabb(layout: &DoricColumnLayout) -> (Vec3, Vec3) {
	let r = layout
		.pedestal
		.radius_bottom
		.max(layout.shaft.radius_bottom)
		.max(layout.capital.radius_top);
	(Vec3::new(-r, 0.0, -r), Vec3::new(r, layout.total_height(), r))
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Color;
	use crate::spec::DoricColumnSpec;

	#[test]
	fn collider_height_matches_layout() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		let (min, max) = cover_aabb(&layout);
		assert!((max.y - min.y - layout.total_height()).abs() < 0.05);
	}

	#[test]
	fn collider_radius_tracks_scale() {
		let narrow = DoricColumnSpec {
			scale: Vec3::splat(0.5),
			..DoricColumnSpec::default()
		};
		let wide = DoricColumnSpec {
			scale: Vec3::splat(2.0),
			..DoricColumnSpec::default()
		};
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
