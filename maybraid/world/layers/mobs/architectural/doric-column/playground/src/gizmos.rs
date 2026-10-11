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
		let parts = [
			(0.0, layout.pedestal.height, layout.pedestal.radius_bottom, layout.pedestal.radius_top),
			(
				layout.pedestal.height,
				layout.shaft.height,
				layout.shaft.radius_bottom,
				layout.shaft.radius_top,
			),
			(
				layout.pedestal.height + layout.shaft.height,
				layout.capital.height,
				layout.capital.radius_bottom,
				layout.capital.radius_top,
			),
		];
		for (base_y, height, r0, r1) in parts {
			draw_cover_cylinder(&mut gizmos, root, base_y, height, (r0 + r1) * 0.5);
		}
	}
	for gt in &anchors {
		gizmos.sphere(gt.translation(), 0.18, Color::srgb(0.15, 0.85, 1.0));
	}
	for gt in &ground {
		gizmos.sphere(gt.translation(), 0.14, Color::srgb(0.2, 1.0, 0.35));
	}
}

fn draw_cover_cylinder(
	gizmos: &mut Gizmos,
	root: &GlobalTransform,
	base_y: f32,
	height: f32,
	radius: f32,
) {
	let center_y = base_y + height * 0.5;
	let center = root.transform_point(Vec3::new(0.0, center_y, 0.0));
	let base = root.transform_point(Vec3::new(0.0, base_y, 0.0));
	let top = root.transform_point(Vec3::new(0.0, base_y + height, 0.0));
	gizmos.line(base, top, Color::srgba(1.0, 0.25, 0.15, 0.95));
	let segments = 8;
	for i in 0..segments {
		let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
		let t1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
		for y in [base_y, base_y + height] {
			let p0 = root.transform_point(Vec3::new(radius * t0.cos(), y, radius * t0.sin()));
			let p1 = root.transform_point(Vec3::new(radius * t1.cos(), y, radius * t1.sin()));
			gizmos.line(p0, p1, Color::srgba(1.0, 0.2, 0.2, 0.55));
		}
		let side0 = root.transform_point(Vec3::new(radius * t0.cos(), base_y, radius * t0.sin()));
		let side1 = root.transform_point(Vec3::new(radius * t0.cos(), base_y + height, radius * t0.sin()));
		gizmos.line(side0, side1, Color::srgba(1.0, 0.2, 0.2, 0.35));
	}
	let _ = center;
}
