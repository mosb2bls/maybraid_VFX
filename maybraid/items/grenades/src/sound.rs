//! Charge SFX: blast impact + rumble, throw jingle, and bounce.

use avian3d::prelude::{CollisionStart, LinearVelocity};
use bevy::prelude::*;
use maybraid_audio::{
	Audio, AudioBus, AudioClip, AudioPlugin, AudioSystems, Mixer, MovementClip, SpatialOneShot,
};

use crate::{GrenadeDetonated, ThrownGrenade};

/// Crack when the charge goes off. Played with [`CHARGE_RUMBLE`].
pub const CHARGE_IMPACT: &str = "sound-effects/environment/explosions/impact_001.wav";
/// Low layer under [`CHARGE_IMPACT`].
pub const CHARGE_RUMBLE: &str = "sound-effects/environment/explosions/rumble_001.wav";
/// Foley for the throw windup.
pub const CHARGE_THROW: &str = "sound-effects/weapons/charges/throw_jingle_001.wav";

pub const IMPACT_VOLUME: f32 = 1.0;
pub const RUMBLE_VOLUME: f32 = 0.85;
pub const THROW_VOLUME: f32 = 1.1;
pub const BOUNCE_VOLUME: f32 = 0.55;
pub const IMPACT_SPATIAL_SCALE: f32 = 0.04;
pub const RUMBLE_SPATIAL_SCALE: f32 = 0.03;
pub const THROW_SPATIAL_SCALE: f32 = 0.35;
pub const BOUNCE_SPATIAL_SCALE: f32 = 0.4;
pub const IMPACT_SPATIAL_RADIUS: f32 = 1.0 / IMPACT_SPATIAL_SCALE;
pub const RUMBLE_SPATIAL_RADIUS: f32 = 1.0 / RUMBLE_SPATIAL_SCALE;
pub const THROW_SPATIAL_RADIUS: f32 = 1.0 / THROW_SPATIAL_SCALE;
pub const BOUNCE_SPATIAL_RADIUS: f32 = 1.0 / BOUNCE_SPATIAL_SCALE;
/// Ignore scrapes; only plant a bounce after a real hit.
pub const BOUNCE_MIN_SPEED: f32 = 1.4;
pub const BOUNCE_COOLDOWN: f32 = 0.14;

/// Loaded charge clips. Missing this resource is a silent no-op.
#[derive(Resource)]
pub struct GrenadeSounds {
	pub impact: Handle<AudioClip>,
	pub rumble: Handle<AudioClip>,
	pub throw: Handle<AudioClip>,
	pub bounce: Handle<AudioClip>,
}

impl GrenadeSounds {
	pub fn load(assets: &AssetServer) -> Self {
		Self {
			impact: assets.load(CHARGE_IMPACT),
			rumble: assets.load(CHARGE_RUMBLE),
			throw: assets.load(CHARGE_THROW),
			bounce: assets.load(MovementClip::Footstep.path(0)),
		}
	}

	pub fn play_detonation(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		world: Vec3,
	) {
		audio.play_or_queue(
			commands,
			&self.impact,
			clips,
			SpatialOneShot::at(world)
				.radius(IMPACT_SPATIAL_RADIUS)
				.gain(IMPACT_VOLUME)
				.bus(AudioBus::Impacts),
			mixer,
			listener,
			"charge-impact",
		);
		audio.play_or_queue(
			commands,
			&self.rumble,
			clips,
			SpatialOneShot::at(world)
				.radius(RUMBLE_SPATIAL_RADIUS)
				.gain(RUMBLE_VOLUME)
				.bus(AudioBus::Impacts),
			mixer,
			listener,
			"charge-rumble",
		);
	}

	pub fn play_throw(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		world: Vec3,
	) {
		audio.play_or_queue(
			commands,
			&self.throw,
			clips,
			SpatialOneShot::at(world)
				.radius(THROW_SPATIAL_RADIUS)
				.gain(THROW_VOLUME)
				.bus(AudioBus::Voices),
			mixer,
			listener,
			"charge-throw",
		);
	}

	pub fn play_bounce(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		world: Vec3,
	) {
		audio.play_or_queue(
			commands,
			&self.bounce,
			clips,
			SpatialOneShot::at(world)
				.radius(BOUNCE_SPATIAL_RADIUS)
				.gain(BOUNCE_VOLUME)
				.bus(AudioBus::Impacts),
			mixer,
			listener,
			"charge-bounce",
		);
	}
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct GrenadeBounceClock {
	pub next_at: f32,
}

pub(crate) fn setup_grenade_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(GrenadeSounds::load(&assets));
}

pub(crate) fn play_grenade_detonations(
	mut commands: Commands,
	mut detonations: MessageReader<GrenadeDetonated>,
	sounds: Option<Res<GrenadeSounds>>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	mut mixer: ResMut<Mixer>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(sounds), Some(audio), Some(listener)) =
		(sounds.as_deref(), audio.as_deref(), listeners.iter().next())
	else {
		return;
	};
	for blast in detonations.read() {
		sounds.play_detonation(&mut commands, &clips, &audio, &mut mixer, listener, blast.position);
	}
}

pub(crate) fn play_grenade_bounces(
	time: Res<Time>,
	mut commands: Commands,
	mut started: MessageReader<CollisionStart>,
	sounds: Option<Res<GrenadeSounds>>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	mut mixer: ResMut<Mixer>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
	mut grenades: Query<
		(&Transform, &LinearVelocity, &mut GrenadeBounceClock),
		With<ThrownGrenade>,
	>,
) {
	let (Some(sounds), Some(audio), Some(listener)) =
		(sounds.as_deref(), audio.as_deref(), listeners.iter().next())
	else {
		return;
	};
	let now = time.elapsed_secs();
	for event in started.read() {
		for candidate in [Some(event.collider1), Some(event.collider2), event.body1, event.body2]
			.into_iter()
			.flatten()
		{
			let Ok((transform, velocity, mut clock)) = grenades.get_mut(candidate) else {
				continue;
			};
			if velocity.0.length() < BOUNCE_MIN_SPEED || now < clock.next_at {
				continue;
			}
			clock.next_at = now + BOUNCE_COOLDOWN;
			sounds.play_bounce(
				&mut commands,
				&clips,
				&audio,
				&mut mixer,
				listener,
				transform.translation,
			);
			break;
		}
	}
}

pub(crate) fn configure_grenade_audio(app: &mut App) {
	if !app.is_plugin_added::<AudioPlugin>() {
		app.add_plugins(AudioPlugin);
	}
	app.add_systems(Startup, setup_grenade_sounds).add_systems(
		PostUpdate,
		(play_grenade_detonations, play_grenade_bounces).before(AudioSystems::Sync),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	fn asset_exists(path: &str) {
		let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(path);
		assert!(file.is_file(), "{}", file.display());
	}

	#[test]
	fn authored_clips_are_in_assets() {
		asset_exists(CHARGE_IMPACT);
		asset_exists(CHARGE_RUMBLE);
		asset_exists(CHARGE_THROW);
		asset_exists(&MovementClip::Footstep.path(0));
	}

	#[test]
	fn authored_spatial_clips_are_mono() {
		for path in [CHARGE_IMPACT, CHARGE_RUMBLE, CHARGE_THROW] {
			let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(path);
			let bytes = std::fs::read(&file).unwrap_or_else(|_| panic!("{}", file.display()));
			assert!(
				maybraid_audio::decode_wav_mono(&bytes).is_ok(),
				"{} must be authored mono",
				file.display()
			);
		}
	}

	#[test]
	fn rumble_carries_farther_than_the_crack() {
		assert!(RUMBLE_SPATIAL_RADIUS > IMPACT_SPATIAL_RADIUS);
		assert!(IMPACT_SPATIAL_RADIUS > THROW_SPATIAL_RADIUS);
		assert!(THROW_SPATIAL_RADIUS > BOUNCE_SPATIAL_RADIUS);
	}
}
