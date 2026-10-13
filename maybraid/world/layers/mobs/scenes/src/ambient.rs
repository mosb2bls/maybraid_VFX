//! Herd ambient: occasional nearby grunts, a wail when a member starts fleeing.

use bevy::prelude::*;
use fleeing_intelligence::FleeingUser;
use maybraid_audio::{
	AmbientPick, AmbientSounds, Audio, AudioClip, AudioPlugin, AudioSystems, Mixer,
};

use crate::MobKind;

pub const HERD_NEAR: f32 = 60.0;
const GRUNT_GAP_MIN: f32 = 6.0;
const GRUNT_GAP_MAX: f32 = 14.0;

#[derive(Resource, Debug)]
pub struct HerdAmbientClock {
	pub next_grunt: f32,
	pub noise: u64,
	pub pick: AmbientPick,
}

impl Default for HerdAmbientClock {
	fn default() -> Self {
		Self { next_grunt: 2.5, noise: 0x51ED_0C0C_77A1_B3E2, pick: AmbientPick::default() }
	}
}

/// Rising-edge latch so a flee writes one wail.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HerdVoice {
	pub wailed: bool,
}

pub(crate) fn ensure_herd_voices(
	mut commands: Commands,
	members: Query<(Entity, &MobKind), Without<HerdVoice>>,
) {
	for (entity, kind) in &members {
		if *kind == MobKind::Herd {
			commands.entity(entity).insert(HerdVoice::default());
		}
	}
}

pub(crate) fn play_herd_ambient(
	time: Res<Time>,
	mut commands: Commands,
	mut clock: ResMut<HerdAmbientClock>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<AmbientSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	mut members: Query<(
		Entity,
		&MobKind,
		&Transform,
		Option<&GlobalTransform>,
		Option<&FleeingUser>,
		Option<&mut HerdVoice>,
	)>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	let origin = listener.translation();
	let mut nearby = Vec::new();
	for (entity, kind, transform, global, fleeing, voice) in &mut members {
		if *kind != MobKind::Herd {
			continue;
		}
		let point = global.map(|g| g.translation()).unwrap_or(transform.translation);
		let fleeing = fleeing.map(FleeingUser::is_driving).unwrap_or(false);
		if fleeing {
			let already = voice.as_ref().is_some_and(|voice| voice.wailed);
			if !already {
				sounds.play_herd_wail(&mut commands, &clips, audio, &mut mixer, listener, point);
				if let Some(mut voice) = voice {
					voice.wailed = true;
				} else {
					commands.entity(entity).insert(HerdVoice { wailed: true });
				}
			}
		} else if let Some(mut voice) = voice {
			voice.wailed = false;
		}
		if planar(origin, point) <= HERD_NEAR {
			nearby.push(point);
		}
	}
	clock.next_grunt = (clock.next_grunt - time.delta_secs()).max(0.0);
	if nearby.is_empty() || clock.next_grunt > 0.0 {
		return;
	}
	let HerdAmbientClock { next_grunt, noise, pick } = &mut *clock;
	let index = (next_u64(noise) as usize) % nearby.len();
	sounds.play_herd_grunt(
		&mut commands,
		pick,
		noise,
		&clips,
		audio,
		&mut mixer,
		listener,
		nearby[index],
	);
	*next_grunt = lerp(GRUNT_GAP_MIN, GRUNT_GAP_MAX, unit(noise));
}

pub(crate) fn configure_herd_ambient(app: &mut App) {
	if !app.is_plugin_added::<AudioPlugin>() {
		app.add_plugins(AudioPlugin);
	}
	app.init_resource::<HerdAmbientClock>().add_systems(
		PostUpdate,
		(ensure_herd_voices, play_herd_ambient)
			.chain()
			.after(TransformSystems::Propagate)
			.before(AudioSystems::Sync),
	);
}

fn planar(a: Vec3, b: Vec3) -> f32 {
	Vec2::new(a.x - b.x, a.z - b.z).length()
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
	min + (max - min) * t
}

fn unit(noise: &mut u64) -> f32 {
	(next_u64(noise) >> 11) as f32 / ((1u64 << 53) as f32)
}

fn next_u64(noise: &mut u64) -> u64 {
	let mut x = if *noise == 0 { 0x9E37_79B9_7F4A_7C15 } else { *noise };
	x ^= x >> 12;
	x ^= x << 25;
	x ^= x >> 27;
	*noise = x;
	x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn herd_near_is_sixty_metres() {
		assert!((HERD_NEAR - 60.0).abs() < 1e-5);
	}

	#[test]
	fn grunt_waits_when_the_herd_is_far() {
		assert!(planar(Vec3::ZERO, Vec3::new(HERD_NEAR + 1.0, 0.0, 0.0)) > HERD_NEAR);
	}
}
