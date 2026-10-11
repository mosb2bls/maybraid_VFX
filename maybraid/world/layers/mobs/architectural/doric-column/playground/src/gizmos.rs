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
		let base = root.translation();
		let top = base + Vec3::Y * layout.total_height();
		let r = layout.shaft.radius_bottom;
		gizmos.line(base, top, Color::srgba(1.0, 0.3, 0.2, 0.9));
		for corner in [
			Vec3::new(r, 0.0, r),
			Vec3::new(-r, 0.0, r),
			Vec3::new(r, 0.0, -r),
			Vec3::new(-r, 0.0, -r),
		] {
			let a = root.transform_point(corner);
			let b = root.transform_point(corner + Vec3::Y * layout.total_height());
			gizmos.line(a, b, Color::srgba(1.0, 0.2, 0.2, 0.35));
		}
	}
	for gt in &anchors {
		gizmos.sphere(gt.translation(), 0.22, Color::srgb(0.2, 0.9, 1.0));
	}
	for gt in &ground {
		gizmos.sphere(gt.translation(), 0.18, Color::srgb(0.2, 1.0, 0.3));
	}
}
