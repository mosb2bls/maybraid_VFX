use bevy::prelude::*;
use doric_column::{
	DoricColumnChainAnchor, DoricColumnGroundContact, DoricColumnRoot, DoricColumnSpec,
};

#[derive(Resource, Debug, Clone, Copy)]
pub struct DrawDoricColumnDebug(pub bool);

impl Default for DrawDoricColumnDebug {
	fn default() -> Self {
		Self(true)
	}
}

pub fn draw_debug(
	mut gizmos: Gizmos,
	enabled: Res<DrawDoricColumnDebug>,
	columns: Query<(&GlobalTransform, &DoricColumnSpec), With<DoricColumnRoot>>,
	anchors: Query<&GlobalTransform, With<DoricColumnChainAnchor>>,
	ground: Query<&GlobalTransform, With<DoricColumnGroundContact>>,
) {
	if !enabled.0 {
		return;
	}
	for (root, spec) in &columns {
		let layout = spec.layout();
		draw_cover_box(
			&mut gizmos,
			root,
			0.0,
			layout.plinth.height,
			layout.plinth.half_side,
		);
		draw_cover_cylinder(
			&mut gizmos,
			root,
			layout.plinth.height,
			layout.shaft.height,
			(layout.shaft.radius_bottom + layout.shaft.radius_top) * 0.5,
		);
		draw_cover_cylinder(
			&mut gizmos,
			root,
			layout.shaft_top_y(),
			layout.echinus.height,
			(layout.echinus.radius_bottom + layout.echinus.radius_top) * 0.5,
		);
		draw_cover_box(
			&mut gizmos,
			root,
			layout.shaft_top_y() + layout.echinus.height,
			layout.abacus.height,
			layout.abacus.half_side,
		);
	}
	for gt in &anchors {
		gizmos.sphere(gt.translation(), 0.18, Color::srgb(0.15, 0.85, 1.0));
	}
	for gt in &ground {
		gizmos.sphere(gt.translation(), 0.14, Color::srgb(0.2, 1.0, 0.35));
	}
}

fn draw_cover_box(
	gizmos: &mut Gizmos,
	root: &GlobalTransform,
	base_y: f32,
	height: f32,
	half: f32,
) {
	let corners = [
		(-half, -half),
		(half, -half),
		(half, half),
		(-half, half),
	];
	for y in [base_y, base_y + height] {
		for i in 0..4 {
			let (x0, z0) = corners[i];
			let (x1, z1) = corners[(i + 1) % 4];
			let p0 = root.transform_point(Vec3::new(x0, y, z0));
			let p1 = root.transform_point(Vec3::new(x1, y, z1));
			gizmos.line(p0, p1, Color::srgba(1.0, 0.2, 0.2, 0.65));
		}
	}
	for (x, z) in corners {
		let p0 = root.transform_point(Vec3::new(x, base_y, z));
		let p1 = root.transform_point(Vec3::new(x, base_y + height, z));
		gizmos.line(p0, p1, Color::srgba(1.0, 0.25, 0.15, 0.45));
	}
}

fn draw_cover_cylinder(
	gizmos: &mut Gizmos,
	root: &GlobalTransform,
	base_y: f32,
	height: f32,
	radius: f32,
) {
	let base = root.transform_point(Vec3::new(0.0, base_y, 0.0));
	let top = root.transform_point(Vec3::new(0.0, base_y + height, 0.0));
	gizmos.line(base, top, Color::srgba(1.0, 0.25, 0.15, 0.95));
	let segments = 12;
	for i in 0..segments {
		let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
		let t1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
		for y in [base_y, base_y + height] {
			let p0 = root.transform_point(Vec3::new(radius * t0.cos(), y, radius * t0.sin()));
			let p1 = root.transform_point(Vec3::new(radius * t1.cos(), y, radius * t1.sin()));
			gizmos.line(p0, p1, Color::srgba(1.0, 0.2, 0.2, 0.55));
		}
	}
}
