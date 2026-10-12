//! Internal 3D render scale: shade at `scale ×` window physical size, present at native size.
//!
//! Scale `1.0` renders the follow camera straight to the window (zero extra cost).
//! Below that, the 3D camera writes an offscreen image that a higher-order `Camera2d`
//! upscales with linear filtering. UI stays on the window-sized present camera.
//!
//! Default is the Retina-friendly `1 / window.scale_factor()` (logical resolution).
//! `MAYBRAID_RENDER_SCALE` and `/render scale <f>` set an explicit factor in
//! [`RenderScale::MIN`]..=[`RenderScale::MAX`].

mod target;

use bevy::prelude::*;

pub use target::{map_render_to_window, map_window_to_render};

/// `MAYBRAID_RENDER_SCALE=0.75` (or `auto`) at process start.
pub const RENDER_SCALE_ENV: &str = "MAYBRAID_RENDER_SCALE";

/// Console / env request to set an explicit factor (already clamped by the applier).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestRenderScale(pub f32);

/// 3D shading scale relative to the window's physical size.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct RenderScale {
	/// `None` derives [`Self::retina_default`] from the live window scale factor.
	requested: Option<f32>,
}

impl Default for RenderScale {
	fn default() -> Self {
		Self { requested: None }
	}
}

impl RenderScale {
	pub const MIN: f32 = 0.5;
	pub const MAX: f32 = 1.0;

	/// Read [`RENDER_SCALE_ENV`] (`auto` / empty → Retina default).
	pub fn from_env() -> Self {
		Self { requested: parse_render_scale_env(std::env::var(RENDER_SCALE_ENV).ok().as_deref()) }
	}

	pub fn requested(&self) -> Option<f32> {
		self.requested
	}

	pub fn is_explicit(&self) -> bool {
		self.requested.is_some()
	}

	pub fn clamp(value: f32) -> f32 {
		if !value.is_finite() {
			return Self::MAX;
		}
		value.clamp(Self::MIN, Self::MAX)
	}

	/// `1 / scale_factor`, clamped. A 2× Retina window renders at logical resolution.
	pub fn retina_default(window_scale_factor: f32) -> f32 {
		if window_scale_factor > 0.0 && window_scale_factor.is_finite() {
			Self::clamp(1.0 / window_scale_factor)
		} else {
			Self::MAX
		}
	}

	pub fn set(&mut self, value: f32) {
		self.requested = Some(Self::clamp(value));
	}

	pub fn clear_to_auto(&mut self) {
		self.requested = None;
	}

	pub fn effective(&self, window_scale_factor: f32) -> f32 {
		self.requested.unwrap_or_else(|| Self::retina_default(window_scale_factor))
	}

	/// Offscreen path is skipped at scale 1.0 so the default 1× display is free.
	pub fn uses_offscreen(&self, window_scale_factor: f32) -> bool {
		self.effective(window_scale_factor) < Self::MAX
	}

	pub fn plan(&self, window_physical: UVec2, window_scale_factor: f32) -> RenderScalePlan {
		let scale = self.effective(window_scale_factor);
		let target_size = target_physical_size(window_physical, scale);
		let window_logical = window_logical_size(window_physical, window_scale_factor);
		RenderScalePlan {
			scale,
			offscreen: scale < Self::MAX && window_physical.x > 0 && window_physical.y > 0,
			target_size,
			image_scale_factor: image_scale_factor(target_size, window_logical),
			window_logical,
		}
	}
}

/// Resolved sizes for one window + scale pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderScalePlan {
	pub scale: f32,
	pub offscreen: bool,
	pub target_size: UVec2,
	/// [`bevy::camera::ImageRenderTarget::scale_factor`] so the 3D camera's
	/// logical viewport matches [`Self::window_logical`].
	pub image_scale_factor: f32,
	pub window_logical: Vec2,
}

/// `ceil(physical × scale)`, at least 1×1 when the window is non-empty.
pub fn target_physical_size(window_physical: UVec2, scale: f32) -> UVec2 {
	if window_physical.x == 0 || window_physical.y == 0 {
		return UVec2::ZERO;
	}
	let scale = RenderScale::clamp(scale);
	UVec2::new(
		((window_physical.x as f32) * scale).ceil().max(1.0) as u32,
		((window_physical.y as f32) * scale).ceil().max(1.0) as u32,
	)
}

pub fn window_logical_size(window_physical: UVec2, window_scale_factor: f32) -> Vec2 {
	let factor = if window_scale_factor > 0.0 && window_scale_factor.is_finite() {
		window_scale_factor
	} else {
		1.0
	};
	Vec2::new(window_physical.x as f32 / factor, window_physical.y as f32 / factor)
}

/// Image-target scale factor that keeps logical viewport == window logical size.
pub fn image_scale_factor(target_physical: UVec2, window_logical: Vec2) -> f32 {
	if target_physical.x == 0 || window_logical.x <= 0.0 {
		return 1.0;
	}
	target_physical.x as f32 / window_logical.x
}

/// Parse `MAYBRAID_RENDER_SCALE`. `auto` / empty → `None` (Retina default).
pub fn parse_render_scale_env(raw: Option<&str>) -> Option<f32> {
	let Some(raw) = raw else {
		return None;
	};
	let raw = raw.trim();
	if raw.is_empty() || raw.eq_ignore_ascii_case("auto") {
		return None;
	}
	match raw.parse::<f32>() {
		Ok(value) if value.is_finite() => Some(RenderScale::clamp(value)),
		Ok(_) => {
			eprintln!("[{RENDER_SCALE_ENV}] non-finite {raw:?} (use 0.5-1.0 or auto)");
			None
		}
		Err(_) => {
			eprintln!("[{RENDER_SCALE_ENV}] unknown {raw:?} (use 0.5-1.0 or auto)");
			None
		}
	}
}

pub(crate) fn configure(app: &mut App) {
	app.insert_resource(RenderScale::from_env())
		.init_resource::<target::AppliedRenderScale>()
		.add_systems(
			Update,
			(apply_render_scale_requests, target::sync_render_scale_target).chain(),
		);
}

pub(crate) fn apply_render_scale_requests(
	mut commands: Commands,
	requests: Query<(Entity, &RequestRenderScale)>,
	mut scale: ResMut<RenderScale>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
) {
	let mut last = None;
	for (entity, request) in &requests {
		let value = RenderScale::clamp(request.0);
		scale.set(value);
		last = Some(value);
		commands.entity(entity).despawn();
	}
	let Some(value) = last else {
		return;
	};
	let line = format!("render scale {value:.3}");
	if let Some(status) = status.as_mut() {
		status.0 = line.clone();
	}
	info!("{line}");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamp_rejects_out_of_range_and_non_finite() -> anyhow::Result<()> {
		assert_eq!(RenderScale::clamp(0.25), RenderScale::MIN);
		assert_eq!(RenderScale::clamp(1.5), RenderScale::MAX);
		assert_eq!(RenderScale::clamp(0.75), 0.75);
		assert_eq!(RenderScale::clamp(f32::NAN), RenderScale::MAX);
		assert_eq!(RenderScale::clamp(f32::INFINITY), RenderScale::MAX);
		Ok(())
	}

	#[test]
	fn retina_default_is_inverse_scale_factor() -> anyhow::Result<()> {
		assert_eq!(RenderScale::retina_default(1.0), 1.0);
		assert_eq!(RenderScale::retina_default(2.0), 0.5);
		assert_eq!(RenderScale::retina_default(3.0), 0.5);
		assert_eq!(RenderScale::retina_default(1.5), RenderScale::clamp(1.0 / 1.5));
		assert_eq!(RenderScale::retina_default(0.0), 1.0);
		Ok(())
	}

	#[test]
	fn explicit_request_wins_over_retina_default() -> anyhow::Result<()> {
		let mut scale = RenderScale::default();
		assert!(!scale.is_explicit());
		assert_eq!(scale.effective(2.0), 0.5);
		assert!(scale.uses_offscreen(2.0));
		assert!(!scale.uses_offscreen(1.0));
		scale.set(1.0);
		assert!(scale.is_explicit());
		assert_eq!(scale.effective(2.0), 1.0);
		assert!(!scale.uses_offscreen(2.0));
		scale.clear_to_auto();
		assert_eq!(scale.effective(2.0), 0.5);
		Ok(())
	}

	#[test]
	fn target_size_ceils_physical_times_scale() -> anyhow::Result<()> {
		assert_eq!(target_physical_size(UVec2::new(2560, 1440), 0.5), UVec2::new(1280, 720));
		assert_eq!(target_physical_size(UVec2::new(1280, 720), 1.0), UVec2::new(1280, 720));
		assert_eq!(target_physical_size(UVec2::new(1921, 1080), 0.5), UVec2::new(961, 540));
		assert_eq!(target_physical_size(UVec2::ZERO, 0.5), UVec2::ZERO);
		Ok(())
	}

	#[test]
	fn retina_plan_renders_at_logical_size() -> anyhow::Result<()> {
		let plan = RenderScale::default().plan(UVec2::new(2560, 1440), 2.0);
		assert_eq!(plan.scale, 0.5);
		assert!(plan.offscreen);
		assert_eq!(plan.target_size, UVec2::new(1280, 720));
		assert_eq!(plan.window_logical, Vec2::new(1280.0, 720.0));
		assert!((plan.image_scale_factor - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn half_scale_on_1x_display_uses_half_image_scale_factor() -> anyhow::Result<()> {
		let mut scale = RenderScale::default();
		scale.set(0.5);
		let plan = scale.plan(UVec2::new(1280, 720), 1.0);
		assert_eq!(plan.target_size, UVec2::new(640, 360));
		assert!((plan.image_scale_factor - 0.5).abs() < 1e-5);
		let logical = plan.target_size.as_vec2() / plan.image_scale_factor;
		assert!((logical - plan.window_logical).abs().max_element() < 1e-3);
		Ok(())
	}

	#[test]
	fn native_scale_skips_offscreen() -> anyhow::Result<()> {
		let mut scale = RenderScale::default();
		scale.set(1.0);
		let plan = scale.plan(UVec2::new(1280, 720), 1.0);
		assert!(!plan.offscreen);
		assert_eq!(plan.scale, 1.0);
		Ok(())
	}

	#[test]
	fn parse_env_clamps_and_treats_auto_as_unset() -> anyhow::Result<()> {
		assert_eq!(parse_render_scale_env(None), None);
		assert_eq!(parse_render_scale_env(Some("")), None);
		assert_eq!(parse_render_scale_env(Some("auto")), None);
		assert_eq!(parse_render_scale_env(Some("0.75")), Some(0.75));
		assert_eq!(parse_render_scale_env(Some("0.1")), Some(0.5));
		assert_eq!(parse_render_scale_env(Some("2")), Some(1.0));
		assert_eq!(parse_render_scale_env(Some("nope")), None);
		Ok(())
	}

	#[test]
	fn apply_request_clamps_and_despawns() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(RenderScale::default());
		world.spawn(RequestRenderScale(0.25));
		world
			.run_system_once(apply_render_scale_requests)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let scale = world.resource::<RenderScale>();
		assert_eq!(scale.requested(), Some(0.5));
		assert_eq!(world.query::<&RequestRenderScale>().iter(&world).count(), 0);
		Ok(())
	}
}
