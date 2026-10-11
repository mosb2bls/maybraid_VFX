//! Nearby blast presence: pad rumble and follow-camera shake.

use std::time::Duration;

use bevy::prelude::*;
use grenades::GrenadeDetonated;
use maybraid_input::PadRumble;
use player_camera::{CameraController, CameraShake};

const SHAKE_MS_NEAR: u64 = 420;
const SHAKE_MS_FAR: u64 = 180;
const RUMBLE_WEAK: f32 = 0.58;
const RUMBLE_STRONG: f32 = 0.86;
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

pub fn pulse_blast_feel(
	mut detonations: MessageReader<GrenadeDetonated>,
	cameras: Query<(&GlobalTransform, Option<&CameraController>), With<Camera3d>>,
	mut shakes: Query<&mut CameraShake, With<Camera3d>>,
	mut rumble: MessageWriter<PadRumble>,
) {
	let Ok((camera, controller)) = cameras.single() else {
		return;
	};
	if controller.is_some_and(|controller| controller.pov.is_map()) {
		return;
	}
	let Ok(mut shake) = shakes.single_mut() else {
		return;
	};
	let origin = camera.translation();
	for blast in detonations.read() {
		let distance = origin.distance(blast.position);
		let shake_presence = blast_presence(distance, blast.radius, blast.effect.scale);
		if shake_presence > 1e-3 {
			shake.kick(shake_presence, 0.22 + 0.28 * shake_presence);
		}
		let pad = rumble_presence(distance, blast.radius, blast.effect.scale);
		if pad > 1e-3 {
			rumble.write(blast_rumble(pad));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use grenades::GrenadeEffect;

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
		assert!(
			feel_radius(10.5, 13.0, RUMBLE_RANGE) > feel_radius(10.5, 13.0, SHAKE_RANGE) * 2.0
		);
	}

	#[test]
	fn closer_blast_is_a_heavier_rumble() {
		let near = blast_rumble(1.0);
		let mid = blast_rumble(0.4);
		assert!(near.duration > mid.duration);
		assert!(near.intensity.strong_motor > mid.intensity.strong_motor);
		assert!(!near.is_silent());
		assert!(blast_rumble(0.0).is_silent());
	}

	#[test]
	fn nearby_detonation_kicks_shake_and_rumble() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<GrenadeDetonated>()
			.add_message::<PadRumble>()
			.add_systems(Update, pulse_blast_feel);
		app.world_mut().spawn((
			Camera3d::default(),
			GlobalTransform::default(),
			CameraShake::default(),
		));
		app.world_mut().write_message(GrenadeDetonated {
			position: Vec3::new(2.0, 0.0, 0.0),
			source: Entity::PLACEHOLDER,
			effect: GrenadeEffect { scale: 13.0, intensity: 1.0, playback: 1.0, seed: None },
			radius: 10.5,
			damage: 65.0,
		});
		app.update();
		let shake = app.world_mut().query::<&CameraShake>().single(app.world()).expect("shake");
		assert!(shake.strength > 0.5);
		assert!(shake.remaining > 0.0);
		let messages = app.world().resource::<Messages<PadRumble>>();
		let mut cursor = messages.get_cursor();
		assert_eq!(cursor.read(messages).count(), 1);
	}
}
