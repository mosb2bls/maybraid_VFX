//! Visual style presentation modes with distinct resolution targets and artistic treatments.
//!
//! Replaces the legacy render scale system. Styles render at characteristic resolutions
//! (preserving aspect ratio) and apply effects during presentation:
//! - **Impressionist** (~1280×720): painterly softening, default
//! - **Sharp**: native resolution, no treatment
//! - **Retro** (~540×304): nearest-neighbor, performance
//! - **Antediluvian** (~400×225): nearest-neighbor, extreme performance
//! - **Pre-post-modern** (~1280×720): misregistered color-print effect
//!
//! UI stays on the window-sized present camera. HUD, menus, and text remain at native resolution.

mod presentation;
mod target;
pub mod visual_style;

pub use presentation::PresentationMaterialsPlugin;

use bevy::prelude::*;

pub use target::{map_render_to_window, map_window_to_render};
pub use visual_style::{PresentationTreatment, VisualStyle, VisualStylePlan};

/// `MAYBRAID_VISUAL_STYLE=retro` (or `impressionist` by default) at process start.
pub const VISUAL_STYLE_ENV: &str = "MAYBRAID_VISUAL_STYLE";
/// Optional override so a capture can keep a style's resolution without its treatment.
pub const PRESENTATION_TREATMENT_ENV: &str = "MAYBRAID_PRESENTATION_TREATMENT";

/// Console / env request to set an explicit visual style.
#[derive(Debug, Clone, Copy)]
pub struct RequestVisualStyle(pub VisualStyle);

impl Message for RequestVisualStyle {}

/// Active visual style setting.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct VisualStyleSetting {
	requested: Option<VisualStyle>,
	treatment_override: Option<PresentationTreatment>,
}

impl Default for VisualStyleSetting {
	fn default() -> Self {
		Self { requested: None, treatment_override: None }
	}
}

impl VisualStyleSetting {
	/// Read [`VISUAL_STYLE_ENV`]. Empty → default [`VisualStyle::Impressionist`].
	pub fn from_env() -> Self {
		Self {
			requested: parse_visual_style_env(std::env::var(VISUAL_STYLE_ENV).ok().as_deref()),
			treatment_override: parse_presentation_treatment_env(
				std::env::var(PRESENTATION_TREATMENT_ENV).ok().as_deref(),
			),
		}
	}

	pub fn requested(&self) -> Option<VisualStyle> {
		self.requested
	}

	pub fn treatment_override(&self) -> Option<PresentationTreatment> {
		self.treatment_override
	}

	pub fn set(&mut self, style: VisualStyle) {
		self.requested = Some(style);
	}

	pub fn set_treatment_override(&mut self, treatment: Option<PresentationTreatment>) {
		self.treatment_override = treatment;
	}

	pub fn clear_to_default(&mut self) {
		self.requested = None;
	}

	pub fn effective(&self) -> VisualStyle {
		self.requested.unwrap_or(VisualStyle::Impressionist)
	}

	pub fn plan(&self, window_physical: UVec2, window_scale_factor: f32) -> VisualStylePlan {
		let mut plan = self.effective().plan(window_physical, window_scale_factor);
		if let Some(treatment) = self.treatment_override {
			plan.treatment = treatment;
		}
		plan
	}
}

/// Parse `MAYBRAID_PRESENTATION_TREATMENT`. Empty → keep the style's treatment.
pub fn parse_presentation_treatment_env(raw: Option<&str>) -> Option<PresentationTreatment> {
	let Some(raw) = raw else {
		return None;
	};
	let raw = raw.trim();
	if raw.is_empty() {
		return None;
	}
	match raw {
		"none" => Some(PresentationTreatment::None),
		"impressionist" => Some(PresentationTreatment::Impressionist),
		"nearest" | "nearest_neighbor" => Some(PresentationTreatment::NearestNeighbor),
		"pre_post_modern" => Some(PresentationTreatment::PrePostModern),
		other => {
			eprintln!(
				"[{PRESENTATION_TREATMENT_ENV}] unknown {other:?} (use none, impressionist, nearest, or pre_post_modern)"
			);
			None
		}
	}
}

/// Parse `MAYBRAID_VISUAL_STYLE`. Empty → `None` (default Impressionist).
pub fn parse_visual_style_env(raw: Option<&str>) -> Option<VisualStyle> {
	let Some(raw) = raw else {
		return None;
	};
	let raw = raw.trim();
	if raw.is_empty() {
		return None;
	}
	match VisualStyle::from_script_id(raw) {
		Some(style) => Some(style),
		None => {
			eprintln!("[{VISUAL_STYLE_ENV}] unknown {raw:?} (use impressionist, sharp, retro, antediluvian, or pre_post_modern)");
			None
		}
	}
}

/// LEGACY: `MAYBRAID_RENDER_SCALE=0.75` (or `auto`) at process start.
pub const RENDER_SCALE_ENV: &str = "MAYBRAID_RENDER_SCALE";

/// LEGACY: Console / env request to set an explicit factor (already clamped by the applier).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestRenderScale(pub f32);

/// LEGACY: 3D shading scale relative to the window's physical size.
/// Preserved for backward compatibility. New code should use [`VisualStyleSetting`].
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
	app.add_plugins(presentation::PresentationMaterialsPlugin)
		.add_message::<RequestVisualStyle>()
		.insert_resource(VisualStyleSetting::from_env())
		.insert_resource(RenderScale::from_env())
		.init_resource::<target::AppliedRenderScale>()
		.add_systems(
			Update,
			(
				apply_visual_style_requests,
				apply_render_scale_requests,
				target::sync_render_scale_target,
			)
				.chain(),
		);
}

/// Read RequestVisualStyle messages and apply them.
pub(crate) fn apply_visual_style_requests(
	mut requests: MessageReader<RequestVisualStyle>,
	mut style: ResMut<VisualStyleSetting>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
) {
	let mut last = None;
	for request in requests.read() {
		style.set(request.0);
		last = Some(request.0);
	}
	let Some(value) = last else {
		return;
	};
	let line = format!("visual style {}", value.label());
	if let Some(status) = status.as_mut() {
		status.0 = line.clone();
	}
	info!("{line}");
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
	fn visual_style_parses_script_ids() -> anyhow::Result<()> {
		assert_eq!(parse_visual_style_env(None), None);
		assert_eq!(parse_visual_style_env(Some("")), None);
		assert_eq!(parse_visual_style_env(Some("impressionist")), Some(VisualStyle::Impressionist));
		assert_eq!(parse_visual_style_env(Some("sharp")), Some(VisualStyle::Sharp));
		assert_eq!(parse_visual_style_env(Some("retro")), Some(VisualStyle::Retro));
		assert_eq!(parse_visual_style_env(Some("antediluvian")), Some(VisualStyle::Antediluvian));
		assert_eq!(
			parse_visual_style_env(Some("pre_post_modern")),
			Some(VisualStyle::PrePostModern)
		);
		assert_eq!(parse_visual_style_env(Some("unknown")), None);
		Ok(())
	}

	#[test]
	fn visual_style_setting_defaults_to_impressionist() -> anyhow::Result<()> {
		let setting = VisualStyleSetting::default();
		assert_eq!(setting.effective(), VisualStyle::Impressionist);
		Ok(())
	}

	#[test]
	fn treatment_override_keeps_style_resolution() -> anyhow::Result<()> {
		let mut setting = VisualStyleSetting::default();
		setting.set(VisualStyle::Impressionist);
		setting.set_treatment_override(Some(PresentationTreatment::None));
		let window = UVec2::new(1920, 1080);
		let plan = setting.plan(window, 1.0);
		assert_eq!(plan.target_size, UVec2::new(1280, 720));
		assert_eq!(plan.treatment, PresentationTreatment::None);
		assert!(plan.requires_offscreen(window));
		assert_eq!(parse_presentation_treatment_env(Some("none")), Some(PresentationTreatment::None));
		Ok(())
	}

	#[test]
	fn apply_visual_style_request_is_consumed() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<Messages<RequestVisualStyle>>();
		world.insert_resource(VisualStyleSetting::default());
		world.write_message(RequestVisualStyle(VisualStyle::Retro));
		world
			.run_system_once(apply_visual_style_requests)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.resource::<VisualStyleSetting>().requested(),
			Some(VisualStyle::Retro)
		);
		Ok(())
	}

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
