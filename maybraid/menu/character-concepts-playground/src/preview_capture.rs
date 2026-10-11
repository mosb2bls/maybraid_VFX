//! Headless / CI capture hooks (env-driven). Does not affect normal playground use.
//!
//! See `maybraid/world/layers/mobs/characters/animations/previews/epiphant-1130/README.md`.

use bevy::prelude::*;
use character_motion::{sample_bray_mouth_emit, BrayMouthEmit};
use characters::{BoneMap, CharacterMembers, CharacterRig, CharacterRigRole};
use game_commands::GameCommandDrawerVisible;

use crate::animation::ConceptAnimation;
use crate::preview::{ConceptPreviewConfig, ConceptPreviewRoot};

/// Boss-scale Bikanwarda assembly (~2 m preview hull × 10 ≈ 20 m).
pub const BOSS_ENCOUNTER_SCALE: f32 = 10.0;

#[derive(Resource, Clone, Copy, Debug)]
pub struct CaptureCameraLocked;

pub fn capture_camera_locked(lock: Option<Res<CaptureCameraLocked>>) -> bool {
	lock.is_some()
}

pub fn capture_camera_unlocked(lock: Option<Res<CaptureCameraLocked>>) -> bool {
	lock.is_none()
}

pub fn capture_hides_creator_ui() -> bool {
	let settings = PreviewCaptureSettings::from_env();
	settings.active() && settings.hide_ui
}

#[derive(Resource, Debug, Clone)]
pub struct PreviewCaptureSettings {
	pub encounter_scale: f32,
	pub sample_progress: Option<f32>,
	pub quit_after_secs: Option<f32>,
	pub interrupt_after_secs: Option<f32>,
	pub interrupt_animation: Option<ConceptAnimation>,
	pub hide_ui: bool,
	pub draw_mouth_emit: bool,
}

impl PreviewCaptureSettings {
	pub fn from_env() -> Self {
		Self {
			encounter_scale: env_parse_f32("CROZON_PREVIEW_ENCOUNTER_SCALE").unwrap_or(1.0),
			sample_progress: env_parse_f32("CROZON_PREVIEW_SAMPLE_PROGRESS"),
			quit_after_secs: env_parse_f32("CROZON_PREVIEW_CAPTURE_QUIT_SECS"),
			interrupt_after_secs: env_parse_f32("CROZON_PREVIEW_INTERRUPT_AFTER_SECS"),
			interrupt_animation: env_parse_animation("CROZON_PREVIEW_INTERRUPT_ANIMATION"),
			hide_ui: env_flag("CROZON_PREVIEW_HIDE_UI").unwrap_or(true),
			draw_mouth_emit: env_flag("CROZON_PREVIEW_DRAW_MOUTH_EMIT").unwrap_or(false),
		}
	}

	pub fn active(&self) -> bool {
		self.encounter_scale != 1.0
			|| self.sample_progress.is_some()
			|| self.quit_after_secs.is_some()
			|| self.interrupt_after_secs.is_some()
			|| self.draw_mouth_emit
	}
}

/// Read once when spawning the preview LodScene root ([`preview::spawn_clothed_character`]).
pub fn encounter_scale_from_env() -> f32 {
	PreviewCaptureSettings::from_env().encounter_scale.max(f32::EPSILON)
}

/// Read when stamping the body clip ([`preview::stamp_lod_character_preview`]).
pub fn sample_progress_from_env() -> Option<f32> {
	PreviewCaptureSettings::from_env().sample_progress
}

#[derive(Resource, Default)]
pub(crate) struct PreviewCaptureInterruptState {
	done: bool,
}

#[derive(Resource, Default)]
pub(crate) struct PendingCaptureInterrupt {
	animation: Option<ConceptAnimation>,
}

pub fn load_preview_capture_settings(mut commands: Commands) {
	let settings = PreviewCaptureSettings::from_env();
	if settings.active() && settings.hide_ui {
		commands.insert_resource(GameCommandDrawerVisible(false));
	}
	if settings.active() {
		commands.insert_resource(settings);
		commands.insert_resource(CaptureCameraLocked);
		commands.init_resource::<PreviewCaptureInterruptState>();
		commands.init_resource::<PendingCaptureInterrupt>();
	}
}

/// Side / 3/4 framing that scales with encounter size (boss ≈ 20 m assembly).
pub fn capture_camera_frame(encounter_scale: f32) -> (Vec3, Vec3) {
	let scale = encounter_scale.max(1.0);
	let center = Vec3::new(0.0, 0.55 * scale, 0.0);
	if scale <= 1.0 + f32::EPSILON {
		let look_at = Vec3::new(0.0, 0.95, 0.0);
		let eye = Vec3::new(2.4, 1.4, 5.2);
		return (eye, look_at);
	}
	let distance = 4.2 * scale;
	let eye = center + Vec3::new(distance * 0.58, distance * 0.42, distance * 0.92);
	(eye, center)
}

pub fn mouth_emit_gizmo_enabled(settings: Option<Res<PreviewCaptureSettings>>) -> bool {
	settings.is_some_and(|s| s.draw_mouth_emit)
}

pub fn preview_capture_interrupt(
	time: Res<Time>,
	mut state: ResMut<PreviewCaptureInterruptState>,
	mut pending: ResMut<PendingCaptureInterrupt>,
	settings: Option<Res<PreviewCaptureSettings>>,
) {
	let Some(settings) = settings else {
		return;
	};
	if state.done {
		return;
	}
	let Some(after) = settings.interrupt_after_secs else {
		return;
	};
	let Some(animation) = settings.interrupt_animation else {
		return;
	};
	if time.elapsed_secs() < after {
		return;
	}
	pending.animation = Some(animation);
	state.done = true;
}

pub fn apply_pending_capture_interrupt(
	mut commands: Commands,
	mut config: ResMut<ConceptPreviewConfig>,
	mut pending: ResMut<PendingCaptureInterrupt>,
	settings: Option<Res<PreviewCaptureSettings>>,
	roots: Query<&CharacterMembers, With<ConceptPreviewRoot>>,
	rigs: Query<&CharacterRig>,
) {
	if settings.is_none() {
		return;
	}
	let Some(animation) = pending.animation.take() else {
		return;
	};
	*config = config.clone().with_animation(animation);
	for members in &roots {
		for member in members.iter() {
			if rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				commands.entity(member).remove::<characters::AnimProgress>();
			}
		}
	}
}

pub fn preview_capture_auto_quit(
	time: Res<Time>,
	settings: Option<Res<PreviewCaptureSettings>>,
	visible: Query<(), With<ConceptPreviewRoot>>,
) {
	let Some(settings) = settings else {
		return;
	};
	let Some(quit) = settings.quit_after_secs else {
		return;
	};
	if visible.is_empty() || time.elapsed_secs() < quit {
		return;
	}
	std::process::exit(0);
}

pub fn draw_preview_mouth_emit_gizmo(
	settings: Option<Res<PreviewCaptureSettings>>,
	config: Res<ConceptPreviewConfig>,
	roots: Query<&CharacterMembers, With<ConceptPreviewRoot>>,
	rigs: Query<(&CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform>,
	mut gizmos: Gizmos,
) {
	let Some(settings) = settings else {
		return;
	};
	if !settings.draw_mouth_emit || !matches!(config.animation(), ConceptAnimation::EpiphantBray) {
		return;
	}
	let scale = settings.encounter_scale.max(1.0);
	let arrow_len = 1.25 * scale;
	let sphere_r = 0.12 * scale.sqrt();
	for members in &roots {
		let maps: Vec<&BoneMap> = members
			.iter()
			.filter_map(|member| rigs.get(member).ok().map(|(_, map)| map))
			.collect();
		let Some(emit) = sample_bray_mouth_emit(&maps, &globals) else {
			continue;
		};
		draw_bray_mouth_emit_gizmo(&mut gizmos, emit, arrow_len, sphere_r);
	}
}

fn draw_bray_mouth_emit_gizmo(
	gizmos: &mut Gizmos,
	emit: BrayMouthEmit,
	arrow_len: f32,
	sphere_r: f32,
) {
	let origin = emit.origin;
	let tip = origin + emit.forward * arrow_len;
	gizmos.sphere(Isometry3d::from_translation(origin), sphere_r, Color::srgb(0.2, 0.95, 1.0));
	gizmos.line(origin, tip, Color::srgb(1.0, 0.92, 0.2));
	let side = Vec3::Y.cross(emit.forward).normalize_or_zero() * (sphere_r * 2.5);
	if side.length_squared() > f32::EPSILON {
		gizmos.line(tip - side, tip + side, Color::srgb(1.0, 0.92, 0.2));
		gizmos.line(tip, tip + Vec3::Y * sphere_r * 1.5, Color::srgb(1.0, 0.92, 0.2));
	}
}

fn env_parse_f32(name: &str) -> Option<f32> {
	std::env::var(name).ok()?.parse().ok()
}

fn env_flag(name: &str) -> Option<bool> {
	let value = std::env::var(name).ok()?.to_ascii_lowercase();
	if matches!(value.as_str(), "1" | "true" | "yes") {
		Some(true)
	} else if matches!(value.as_str(), "0" | "false" | "no") {
		Some(false)
	} else {
		None
	}
}

fn env_parse_animation(name: &str) -> Option<ConceptAnimation> {
	let raw = std::env::var(name).ok()?;
	ConceptAnimation::VALUES.iter().copied().find(|value| value.label() == raw)
}
