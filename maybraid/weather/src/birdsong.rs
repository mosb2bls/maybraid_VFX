//! Periodic birdsong around the listener. Independent of live herds.

use std::f32::consts::TAU;

use bevy::prelude::*;
use maybraid_audio::{AmbientPick, AmbientSounds, Audio, AudioClip, Mixer};

use crate::wind;

const BIRDSONG_GAP_MIN: f32 = 10.0;
const BIRDSONG_GAP_MAX: f32 = 22.0;
const BIRDSONG_LIFT: f32 = 3.4;
/// Occasional close song; most samples land past this.
const BIRDSONG_NEAR_MIN: f32 = 12.0;
const BIRDSONG_FAR_MIN: f32 = 20.0;
const BIRDSONG_FAR_MAX: f32 = 42.0;
const BIRDSONG_FAR_CHANCE: f32 = 0.8;

/// When the next song may play.
#[derive(Resource, Debug)]
pub struct BirdsongClock {
	pub next: f32,
	pub noise: u64,
	pub pick: AmbientPick,
}

impl Default for BirdsongClock {
	fn default() -> Self {
		Self { next: 3.0, noise: 0xA5A5_C3C3_1F2E_8D90, pick: AmbientPick::default() }
	}
}

pub(crate) fn spawn_birdsong_near_listener(
	time: Res<Time>,
	mut commands: Commands,
	mut clock: ResMut<BirdsongClock>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<AmbientSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	clock.next = (clock.next - time.delta_secs()).max(0.0);
	if clock.next > 0.0 {
		return;
	}
	let origin = listener.translation();
	let BirdsongClock { next, noise, pick } = &mut *clock;
	let world = point_birdsong(origin, noise);
	sounds.play_birdsong(&mut commands, pick, noise, &clips, audio, &mut mixer, listener, world);
	*next = wind::lerp(BIRDSONG_GAP_MIN, BIRDSONG_GAP_MAX, wind::unit(noise));
}

fn point_birdsong(origin: Vec3, noise: &mut u64) -> Vec3 {
	let angle = wind::unit(noise) * TAU;
	let dist = planar_dist(noise);
	origin + Vec3::new(angle.cos() * dist, BIRDSONG_LIFT, angle.sin() * dist)
}

fn planar_dist(noise: &mut u64) -> f32 {
	if wind::unit(noise) < BIRDSONG_FAR_CHANCE {
		wind::lerp(BIRDSONG_FAR_MIN, BIRDSONG_FAR_MAX, wind::unit(noise))
	} else {
		wind::lerp(BIRDSONG_NEAR_MIN, BIRDSONG_FAR_MIN, wind::unit(noise))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn first_song_is_not_immediate() {
		let clock = BirdsongClock::default();
		assert!(clock.next > 0.0);
		assert!(clock.next < BIRDSONG_GAP_MIN);
	}

	#[test]
	fn most_songs_are_twenty_metres_out() {
		let origin = Vec3::new(3.0, 1.0, -2.0);
		let mut noise = 11;
		let mut far = 0;
		for _ in 0..80 {
			let point = point_birdsong(origin, &mut noise);
			let planar = Vec2::new(point.x - origin.x, point.z - origin.z).length();
			assert!(planar >= BIRDSONG_NEAR_MIN - 1e-3);
			assert!(planar <= BIRDSONG_FAR_MAX + 1e-3);
			assert!((point.y - origin.y - BIRDSONG_LIFT).abs() < 1e-4);
			if planar >= BIRDSONG_FAR_MIN - 1e-3 {
				far += 1;
			}
		}
		assert!(far >= 56);
	}
}
