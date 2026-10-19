//! Nearby blast presence: spatial-rumble camera and pad sources.

use std::time::Duration;

use bevy::prelude::*;
use grenades::GrenadeDetonated;
use maybraid_input::PadRumble;
use maybraid_spatial_rumble::{
	CameraRumbleShape, GroundRumbleShape, PadRumbleShape, RumbleChannels, RumbleEnvelope,
	RumbleFalloff, SpatialRumble,
};
use player_camera::CameraController;

const SHAKE_MS_NEAR: u64 = 420;
const SHAKE_MS_FAR: u64 = 180;
const RUMBLE_WEAK: f32 = 0.58;
const RUMBLE_STRONG: f32 = 0.86;
/// Peak camera translation at full presence, matching the old shake amplitude.
const CAMERA_TRANSLATION: f32 = 0.09;
const CAMERA_FREQUENCY: f32 = 8.0;
const CAMERA_DURATION_FAR: f32 = 0.22;
const CAMERA_DURATION_SPAN: f32 = 0.28;
/// Camera shake stays on the visible fireball.
const SHAKE_RANGE: f32 = 2.2;
/// Pad rumble keeps going after the screen kick fades — a few streets over.
const RUMBLE_RANGE: f32 = 6.6;

fn feel_radius(blast_radius: f32, effect_scale: f32, range: f32) -> f32 {
	blast_radius.max(effect_scale * 0.7) * range
}

fn falloff(distance: f32, radius: f32) -> f32 {
	(1.0 - distance / radius.max(1.0)).clamp(0.0, 1.0)
}

/// How far the visible explosion still reads as a nearby hit.
pub fn blast_presence(distance: f32, blast_radius: f32, effect_scale: f32) -> f32 {
	falloff(distance, feel_radius(blast_radius, effect_scale, SHAKE_RANGE))
}

/// Pad motors keep a longer tail than camera shake.
pub fn rumble_presence(distance: f32, blast_radius: f32, effect_scale: f32) -> f32 {
	falloff(distance, feel_radius(blast_radius, effect_scale, RUMBLE_RANGE))
}

pub fn blast_rumble(presence: f32) -> PadRumble {
	let presence = presence.clamp(0.0, 1.0);
	let ms = SHAKE_MS_FAR as f32 + (SHAKE_MS_NEAR - SHAKE_MS_FAR) as f32 * presence;
	PadRumble::motors(
		Duration::from_millis(ms.round() as u64),
		RUMBLE_WEAK * presence,
		RUMBLE_STRONG * presence,
	)
}

/// Grenade detonation → [`SpatialRumble`] camera and pad sources.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrenadeBlastFeel {
	pub origin: Vec3,
	pub blast_radius: f32,
	pub effect_scale: f32,
	pub seed: Option<u64>,
}

impl GrenadeBlastFeel {
	pub fn from_detonation(blast: &GrenadeDetonated) -> Self {
		Self {
			origin: blast.position,
			blast_radius: blast.radius,
			effect_scale: blast.effect.scale,
			seed: blast.effect.seed,
		}
	}

	pub fn shake_radius(self) -> f32 {
		feel_radius(self.blast_radius, self.effect_scale, SHAKE_RANGE)
	}

	pub fn rumble_radius(self) -> f32 {
		feel_radius(self.blast_radius, self.effect_scale, RUMBLE_RANGE)
	}

	pub fn camera_duration(self, presence: f32) -> f32 {
		CAMERA_DURATION_FAR + CAMERA_DURATION_SPAN * presence.clamp(0.0, 1.0)
	}

	/// Camera-only source. Linear falloff uses [`Self::shake_radius`]; silent past the feel ring.
	pub fn camera_source_at(self, listener: Vec3) -> Option<SpatialRumble> {
		let distance = self.origin.distance(listener);
		let presence = blast_presence(distance, self.blast_radius, self.effect_scale);
		if presence <= 1e-3 {
			return None;
		}
		let duration = self.camera_duration(presence);
		Some(SpatialRumble {
			origin: self.origin,
			intensity: 1.0,
			duration,
			envelope: RumbleEnvelope::new(0.0, duration),
			falloff: RumbleFalloff::linear(0.0, self.shake_radius().max(1.0)),
			seed: self.seed,
			channels: RumbleChannels::CAMERA,
			camera: CameraRumbleShape {
				translation: CAMERA_TRANSLATION,
				roll: 0.0,
				frequency: CAMERA_FREQUENCY,
			},
			pad: PadRumbleShape { weak: 0.0, strong: 0.0 },
			ground: GroundRumbleShape { amplitude: 0.0, frequency: 0.0 },
		})
	}

	/// Pad-only source. Linear falloff uses [`Self::rumble_radius`]; longer than the screen kick.
	pub fn pad_source_at(self, listener: Vec3) -> Option<SpatialRumble> {
		let distance = self.origin.distance(listener);
		let presence = rumble_presence(distance, self.blast_radius, self.effect_scale);
		if presence <= 1e-3 {
			return None;
		}
		let pulse = blast_rumble(presence);
		Some(SpatialRumble {
			origin: self.origin,
			intensity: 1.0,
			duration: pulse.duration.as_secs_f32(),
			envelope: RumbleEnvelope::new(0.0, 0.0),
			falloff: RumbleFalloff::linear(0.0, self.rumble_radius().max(1.0)),
			seed: self.seed,
			channels: RumbleChannels::PAD,
			camera: CameraRumbleShape { translation: 0.0, roll: 0.0, frequency: 0.0 },
			pad: PadRumbleShape { weak: RUMBLE_WEAK, strong: RUMBLE_STRONG },
			ground: GroundRumbleShape { amplitude: 0.0, frequency: 0.0 },
		})
	}
}

pub fn pulse_blast_feel(
	mut detonations: MessageReader<GrenadeDetonated>,
	cameras: Query<(&GlobalTransform, Option<&CameraController>), With<Camera3d>>,
	mut rumble: MessageWriter<SpatialRumble>,
) {
	let Ok((camera, controller)) = cameras.single() else {
		return;
	};
	if controller.is_some_and(|controller| controller.pov.is_map()) {
		return;
	}
	let listener = camera.translation();
	for blast in detonations.read() {
		let feel = GrenadeBlastFeel::from_detonation(blast);
		if let Some(camera) = feel.camera_source_at(listener) {
			rumble.write(camera);
		}
		if let Some(pad) = feel.pad_source_at(listener) {
			rumble.write(pad);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use grenades::GrenadeEffect;
	use maybraid_spatial_rumble::{
		AppliedCameraRumble, RumbleFalloffCurve, SpatialRumbleListener, SpatialRumblePlugin,
		SpatialRumbleSource, SpatialRumbleSystems,
	};
	use player_camera::{CameraController, CameraPov};

	fn sample_blast(position: Vec3) -> GrenadeDetonated {
		GrenadeDetonated {
			position,
			source: Entity::PLACEHOLDER,
			effect: GrenadeEffect { scale: 13.0, intensity: 1.0, playback: 1.0, seed: Some(11) },
			radius: 10.5,
			damage: 65.0,
		}
	}

	fn collect_spatial(world: &World) -> Result<Vec<SpatialRumble>> {
		let messages = world
			.get_resource::<Messages<SpatialRumble>>()
			.ok_or_else(|| anyhow::anyhow!("SpatialRumble messages"))?;
		let mut cursor = messages.get_cursor();
		Ok(cursor.read(messages).copied().collect())
	}

	fn collect_pad(world: &World) -> Result<Vec<PadRumble>> {
		let messages = world
			.get_resource::<Messages<PadRumble>>()
			.ok_or_else(|| anyhow::anyhow!("PadRumble messages"))?;
		let mut cursor = messages.get_cursor();
		Ok(cursor.read(messages).copied().collect())
	}

	#[test]
	fn presence_is_full_at_the_origin_and_zero_past_the_feel_ring() {
		assert!((blast_presence(0.0, 10.5, 13.0) - 1.0).abs() < 1e-4);
		assert_eq!(blast_presence(80.0, 10.5, 13.0), 0.0);
		assert!(blast_presence(8.0, 10.5, 13.0) > blast_presence(20.0, 10.5, 13.0));
	}

	#[test]
	fn rumble_reaches_past_the_screen_kick() {
		assert!((rumble_presence(0.0, 10.5, 13.0) - 1.0).abs() < 1e-4);
		assert!(rumble_presence(40.0, 10.5, 13.0) > 0.0);
		assert_eq!(blast_presence(40.0, 10.5, 13.0), 0.0);
		assert!(feel_radius(10.5, 13.0, RUMBLE_RANGE) > feel_radius(10.5, 13.0, SHAKE_RANGE) * 2.0);
	}

	#[test]
	fn closer_blast_is_a_heavier_rumble() {
		let near = blast_rumble(1.0);
		let mid = blast_rumble(0.4);
		assert!(near.duration > mid.duration);
		assert!(near.intensity.strong_motor > mid.intensity.strong_motor);
		assert!(!near.is_silent());
		assert!(blast_rumble(0.0).is_silent());
		assert!(blast_rumble(1.0).intensity.weak_motor <= 1.0);
		assert!(blast_rumble(1.0).intensity.strong_motor <= 1.0);
		assert!(blast_rumble(2.0).intensity.strong_motor <= 1.0);
	}

	#[test]
	fn adapter_falloff_matches_presence_and_keeps_split_ranges() -> Result<()> {
		let feel = GrenadeBlastFeel::from_detonation(&sample_blast(Vec3::ZERO));
		assert!(feel.rumble_radius() > feel.shake_radius() * 2.0);

		let camera = feel
			.camera_source_at(Vec3::ZERO)
			.ok_or_else(|| anyhow::anyhow!("camera source at origin"))?;
		assert_eq!(camera.channels, RumbleChannels::CAMERA);
		assert_eq!(camera.falloff.curve, RumbleFalloffCurve::Linear);
		assert!((camera.intensity - 1.0).abs() < 1e-5);
		assert!((camera.camera.translation - CAMERA_TRANSLATION).abs() < 1e-5);
		assert!(camera.camera.roll.abs() < 1e-5);
		let live = SpatialRumbleSource::from_spec(camera, 11);
		assert!((live.strength_at(Vec3::ZERO) - 1.0).abs() < 1e-4);
		assert!((live.strength_at(Vec3::X * 8.0) - blast_presence(8.0, 10.5, 13.0)).abs() < 1e-4);
		assert_eq!(live.strength_at(Vec3::X * 80.0), 0.0);
		assert!(feel.camera_source_at(Vec3::X * 40.0).is_none());
		assert!(feel.camera_source_at(Vec3::X * 80.0).is_none());

		let pad = feel
			.pad_source_at(Vec3::X * 40.0)
			.ok_or_else(|| anyhow::anyhow!("pad source past the shake ring"))?;
		assert_eq!(pad.channels, RumbleChannels::PAD);
		assert_eq!(pad.falloff.curve, RumbleFalloffCurve::Linear);
		assert!((pad.pad.weak - RUMBLE_WEAK).abs() < 1e-5);
		assert!((pad.pad.strong - RUMBLE_STRONG).abs() < 1e-5);
		let pad_live = SpatialRumbleSource::from_spec(pad, 11);
		assert!(
			(pad_live.strength_at(Vec3::X * 40.0) - rumble_presence(40.0, 10.5, 13.0)).abs() < 1e-4
		);
		assert!(feel.pad_source_at(Vec3::X * 80.0).is_none());
		Ok(())
	}

	#[test]
	fn nearby_detonation_emits_camera_and_pad_sources() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<GrenadeDetonated>()
			.add_message::<SpatialRumble>()
			.add_systems(Update, pulse_blast_feel);
		app.world_mut().spawn((
			Camera3d::default(),
			GlobalTransform::default(),
			Transform::default(),
		));
		app.world_mut().write_message(sample_blast(Vec3::new(2.0, 0.0, 0.0)));
		app.update();
		let messages = collect_spatial(app.world())?;
		assert_eq!(messages.len(), 2);
		assert!(messages.iter().any(|spec| spec.channels == RumbleChannels::CAMERA));
		assert!(messages.iter().any(|spec| spec.channels == RumbleChannels::PAD));
		assert!(messages.iter().all(|spec| spec.intensity <= 1.0));
		Ok(())
	}

	#[test]
	fn map_pov_does_not_emit_sources() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<GrenadeDetonated>()
			.add_message::<SpatialRumble>()
			.add_systems(Update, pulse_blast_feel);
		app.world_mut().spawn((
			Camera3d::default(),
			GlobalTransform::default(),
			Transform::default(),
			CameraController { pov: CameraPov::Map, ..default() },
		));
		app.world_mut().write_message(sample_blast(Vec3::ZERO));
		app.update();
		assert!(collect_spatial(app.world())?.is_empty());
		Ok(())
	}

	#[test]
	fn nearby_detonation_presents_through_spatial_rumble() -> Result<()> {
		use bevy::time::TimeUpdateStrategy;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins(SpatialRumblePlugin)
			.add_message::<GrenadeDetonated>()
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
				1.0 / 60.0,
			)))
			.add_systems(Update, pulse_blast_feel.before(SpatialRumbleSystems::Ingest));
		let camera = app
			.world_mut()
			.spawn((
				Camera3d::default(),
				Transform::default(),
				GlobalTransform::default(),
				SpatialRumbleListener::default().rewriting_pose(),
			))
			.id();
		app.world_mut().write_message(sample_blast(Vec3::new(2.0, 0.0, 0.0)));
		app.update();
		app.update();
		let applied = *app
			.world()
			.get::<AppliedCameraRumble>(camera)
			.ok_or_else(|| anyhow::anyhow!("applied camera rumble"))?;
		assert!(applied.translation.length() > 1e-4);
		assert!(!collect_pad(app.world())?.is_empty());
		Ok(())
	}
}
