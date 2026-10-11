//! Headless / CI capture hooks (env-driven). Does not affect normal playground use.
//!
//! See `maybraid/world/layers/mobs/characters/animations/previews/epiphant-1130/README.md`.

use bevy::prelude::*;

use crate::animation::ConceptAnimation;
use crate::preview::{ConceptPreviewConfig, ConceptPreviewRoot};

/// Boss-scale Bikanwarda assembly (~2 m preview hull × 10 ≈ 20 m).
pub const BOSS_ENCOUNTER_SCALE: f32 = 10.0;

#[derive(Resource, Debug, Clone)]
pub struct PreviewCaptureSettings {
	pub encounter_scale: f32,
	pub sample_progress: Option<f32>,
	pub quit_after_secs: Option<f32>,
	pub interrupt_after_secs: Option<f32>,
	pub interrupt_animation: Option<ConceptAnimation>,
}

impl PreviewCaptureSettings {
	pub fn from_env() -> Self {
		Self {
			encounter_scale: env_parse_f32("CROZON_PREVIEW_ENCOUNTER_SCALE").unwrap_or(1.0),
			sample_progress: env_parse_f32("CROZON_PREVIEW_SAMPLE_PROGRESS"),
			quit_after_secs: env_parse_f32("CROZON_PREVIEW_CAPTURE_QUIT_SECS"),
			interrupt_after_secs: env_parse_f32("CROZON_PREVIEW_INTERRUPT_AFTER_SECS"),
			interrupt_animation: env_parse_animation("CROZON_PREVIEW_INTERRUPT_ANIMATION"),
		}
	}

	pub fn active(&self) -> bool {
		self.encounter_scale != 1.0
			|| self.sample_progress.is_some()
			|| self.quit_after_secs.is_some()
			|| self.interrupt_after_secs.is_some()
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

pub fn load_preview_capture_settings(mut commands: Commands) {
	let settings = PreviewCaptureSettings::from_env();
	if settings.active() {
		commands.insert_resource(settings);
		commands.init_resource::<PreviewCaptureInterruptState>();
	}
}

pub fn adjust_capture_camera_from_env(mut cameras: Query<&mut Transform, With<Camera3d>>) {
	let scale = PreviewCaptureSettings::from_env().encounter_scale;
	if (scale - 1.0).abs() < f32::EPSILON {
		return;
	}
	for mut transform in &mut cameras {
		transform.translation *= scale;
	}
}

pub fn preview_capture_interrupt(
	time: Res<Time>,
	mut config: ResMut<ConceptPreviewConfig>,
	mut state: ResMut<PreviewCaptureInterruptState>,
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
	*config = config.clone().with_animation(animation);
	state.done = true;
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

fn env_parse_f32(name: &str) -> Option<f32> {
	std::env::var(name).ok()?.parse().ok()
}

fn env_parse_animation(name: &str) -> Option<ConceptAnimation> {
	let raw = std::env::var(name).ok()?;
	ConceptAnimation::VALUES.iter().copied().find(|value| value.label() == raw)
}
