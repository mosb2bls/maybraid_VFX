//! World-to-screen projection for HUD pins that clamp to the viewport edge.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};

/// Projects world positions to screen space and clamps off-screen pins to the viewport edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenPin;

impl ScreenPin {
	/// Clamps `point` to the axis-aligned rectangle `[min, max]` along the ray from `center`.
	pub fn clamp_to_rect(center: Vec2, point: Vec2, min: Vec2, max: Vec2) -> Vec2 {
		let dir = point - center;
		if dir.length_squared() < 1e-6 {
			return Vec2::new(center.x.clamp(min.x, max.x), center.y.clamp(min.y, max.y));
		}
		let mut t = f32::INFINITY;
		if dir.x.abs() > 1e-6 {
			let edge = if dir.x > 0.0 { max.x } else { min.x };
			t = t.min((edge - center.x) / dir.x);
		}
		if dir.y.abs() > 1e-6 {
			let edge = if dir.y > 0.0 { max.y } else { min.y };
			t = t.min((edge - center.y) / dir.y);
		}
		center + dir * t.clamp(0.0, 1.0)
	}

	/// Projects `world` through `camera`, clamping to the inset viewport when off-screen.
	///
	/// Returns screen position and whether the point is fully inside the viewport.
	pub fn project(
		camera: &Camera,
		camera_transform: &GlobalTransform,
		world: Vec3,
		margin_px: f32,
	) -> Option<(Vec2, bool)> {
		let rect = camera.logical_viewport_rect()?;
		let mut ndc = camera.world_to_ndc(camera_transform, world)?;
		let in_frustum = ndc.z > 0.0 && ndc.z < 1.0;
		if !in_frustum {
			ndc.x = -ndc.x;
			ndc.y = -ndc.y;
		}
		ndc.y = -ndc.y;
		let mut screen = (ndc.truncate() + Vec2::ONE) / 2.0 * rect.size() + rect.min;
		let on_screen = in_frustum
			&& screen.x >= rect.min.x
			&& screen.x <= rect.max.x
			&& screen.y >= rect.min.y
			&& screen.y <= rect.max.y;
		if !on_screen {
			screen = Self::clamp_to_rect(
				rect.center(),
				screen,
				rect.min + Vec2::splat(margin_px),
				rect.max - Vec2::splat(margin_px),
			);
		}
		Some((screen, on_screen))
	}

	/// Maps a logical viewport point from `from` into `to` (scale + origin).
	///
	/// Used when the gameplay camera's render target is not the UI camera's
	/// window — [PR #1187](https://github.com/ramate-io/maybraid/pull/1187)
	/// offscreen 3D pass vs window present camera.
	pub fn map_between_viewports(point: Vec2, from: Rect, to: Rect) -> Vec2 {
		let from_size = from.size();
		if from_size.x.abs() < 1e-6 || from_size.y.abs() < 1e-6 {
			return point;
		}
		let t = (point - from.min) / from_size;
		to.min + t * to.size()
	}

	/// [`UiGlobalTransform`] is the node **center** in **physical** pixels.
	/// [`Node::left`] / [`Node::top`] want the **top-left** in **logical** pixels.
	pub fn ui_logical_top_left(transform: &UiGlobalTransform, computed: &ComputedNode) -> Vec2 {
		let scale = computed.inverse_scale_factor();
		let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
		let center = transform.affine().translation;
		(center - computed.size() * 0.5) * scale
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Affine2;

	#[test]
	fn clamp_hits_the_near_edge() {
		let clamped = ScreenPin::clamp_to_rect(
			Vec2::new(100.0, 100.0),
			Vec2::new(400.0, 100.0),
			Vec2::splat(20.0),
			Vec2::splat(180.0),
		);
		assert!((clamped.x - 180.0).abs() < 1e-3);
		assert!((clamped.y - 100.0).abs() < 1e-3);
	}

	#[test]
	fn clamp_keeps_an_interior_point() {
		let clamped = ScreenPin::clamp_to_rect(
			Vec2::new(100.0, 100.0),
			Vec2::new(120.0, 110.0),
			Vec2::splat(20.0),
			Vec2::splat(180.0),
		);
		assert!((clamped.x - 120.0).abs() < 1e-3);
		assert!((clamped.y - 110.0).abs() < 1e-3);
	}

	#[test]
	fn map_between_viewports_scales_half_res_3d_into_the_window() {
		let render = Rect::from_corners(Vec2::ZERO, Vec2::new(640.0, 360.0));
		let window = Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0));
		let mapped = ScreenPin::map_between_viewports(Vec2::new(320.0, 180.0), render, window);
		assert!((mapped - Vec2::new(640.0, 360.0)).length() < 1e-3);
		assert_eq!(
			ScreenPin::map_between_viewports(Vec2::new(100.0, 40.0), window, window),
			Vec2::new(100.0, 40.0)
		);
	}

	#[test]
	fn ui_logical_top_left_converts_hidpi_physical_center() {
		let computed =
			ComputedNode { size: Vec2::new(2.0, 2.0), inverse_scale_factor: 0.5, ..default() };
		let transform = UiGlobalTransform::from(Affine2::from_translation(Vec2::new(96.0, 1200.0)));
		let logical = ScreenPin::ui_logical_top_left(&transform, &computed);
		assert!(
			(logical - Vec2::new(47.5, 599.5)).length() < 1e-3,
			"{logical:?} should be logical top-left, not physical center"
		);
		assert!(
			logical.y < 720.0,
			"Retina physical Y must not be used as Node::top on a 720-logical window"
		);
	}
}
