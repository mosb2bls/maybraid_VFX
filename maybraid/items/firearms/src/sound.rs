//! Four-layer firearm SFX: fire, hammer, fizz, and impact.
//!
//! Fire and hammer are world-fixed muzzle one-shots. Bolts and bullets carry a
//! looping fizz that follows the projectile with its simulation velocity. A hit
//! is a world-fixed impact. A laser loops fire on the beam and plays hammer
//! once. Distance is oddio's radius. Mixing and CPAL live in [`maybraid_audio`].

use bevy::prelude::*;
use maybraid_audio::{
	listener, Audio, AudioBus, AudioClip, AudioVelocity, Mixer, SpatialEmitter, SpatialOneShot,
};

use firearms_components::AssetPath;

/// Energy report at the muzzle.
pub const WEAPON_FIRE: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/fire__wet_lazer_001.wav");
/// Percussive layer fired with [`WEAPON_FIRE`].
pub const WEAPON_HAMMER: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/hammer__lazer_001.wav");
/// Seamless flight loop on a bolt or bullet.
pub const WEAPON_FIZZ: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/fizz__lazer_001.wav");
/// One-shot when a bolt or bullet first crosses a collider.
pub const WEAPON_IMPACT: AssetPath =
	AssetPath::new("sound-effects/weapons/firearms/impact__lazer_001.wav");

pub const FIRE_VOLUME: f32 = 0.7;
pub const HAMMER_VOLUME: f32 = 1.0;
pub const FIZZ_VOLUME: f32 = 1.3;
pub const IMPACT_VOLUME: f32 = 0.8;
/// Inverse of the oddio zero-attenuation radius (meters).
pub const FIRE_SPATIAL_SCALE: f32 = 0.05;
pub const HAMMER_SPATIAL_SCALE: f32 = 0.1;
pub const FIZZ_SPATIAL_SCALE: f32 = 0.2;
/// Impact carries farther than in-flight fizz (8 m vs 5 m).
pub const IMPACT_SPATIAL_SCALE: f32 = 0.125;
pub const FIRE_SPATIAL_RADIUS: f32 = 1.0 / FIRE_SPATIAL_SCALE;
pub const HAMMER_SPATIAL_RADIUS: f32 = 1.0 / HAMMER_SPATIAL_SCALE;
pub const FIZZ_SPATIAL_RADIUS: f32 = 1.0 / FIZZ_SPATIAL_SCALE;
pub const IMPACT_SPATIAL_RADIUS: f32 = 1.0 / IMPACT_SPATIAL_SCALE;
pub const FIRE_LISTENER_GAP: f32 = maybraid_audio::FIRE_LISTENER_GAP;
/// Laser mesh is a cylinder centered on the beam. Parent scale.y is length, so
/// local −0.5 Y sits on the muzzle.
pub const LASER_MUZZLE_LOCAL: Vec3 = Vec3::new(0.0, -0.5, 0.0);

/// Looping fizz emitter parented to a bolt or bullet.
#[derive(Component, Clone, Copy, Debug)]
pub struct FlightFizz;

/// Loaded fire, hammer, fizz, and impact clips. Missing this resource is a silent no-op.
#[derive(Resource)]
pub struct FirearmFireSounds {
	pub fire: Handle<AudioClip>,
	pub hammer: Handle<AudioClip>,
	pub fizz: Handle<AudioClip>,
	pub impact: Handle<AudioClip>,
}

impl FirearmFireSounds {
	pub fn load(asset_server: &AssetServer) -> Self {
		Self {
			fire: asset_server.load(WEAPON_FIRE.as_str()),
			hammer: asset_server.load(WEAPON_HAMMER.as_str()),
			fizz: asset_server.load(WEAPON_FIZZ.as_str()),
			impact: asset_server.load(WEAPON_IMPACT.as_str()),
		}
	}

	pub fn listener() -> SpatialListener {
		listener()
	}

	pub fn play_shot(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		world: Vec3,
		player: bool,
		delay: f32,
		duck: bool,
	) {
		let bus = if player { AudioBus::PlayerWeapon } else { AudioBus::Weapons };
		if duck && player {
			mixer.duck_player_shot();
		}
		audio.play_or_queue(
			commands,
			&self.fire,
			clips,
			SpatialOneShot::at(world)
				.radius(FIRE_SPATIAL_RADIUS)
				.gain(FIRE_VOLUME)
				.bus(bus)
				.delay(delay)
				.duck(false),
			mixer,
			listener,
			"firearm-fire",
		);
		audio.play_or_queue(
			commands,
			&self.hammer,
			clips,
			SpatialOneShot::at(world)
				.radius(HAMMER_SPATIAL_RADIUS)
				.gain(HAMMER_VOLUME)
				.bus(bus)
				.delay(delay)
				.duck(false),
			mixer,
			listener,
			"firearm-hammer",
		);
	}

	pub fn loop_on(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		laser: Entity,
		world: Vec3,
		player: bool,
	) {
		let bus = if player { AudioBus::PlayerWeapon } else { AudioBus::Weapons };
		commands.entity(laser).insert(AudioVelocity(Vec3::ZERO));
		audio.play_loop_or_queue(
			commands,
			&self.fire,
			clips,
			SpatialEmitter::on(laser, world)
				.radius(FIRE_SPATIAL_RADIUS)
				.gain(FIRE_VOLUME)
				.bus(bus),
			mixer,
			listener,
			"firearm-fire",
		);
		audio.play_or_queue(
			commands,
			&self.hammer,
			clips,
			SpatialOneShot::at(world)
				.radius(HAMMER_SPATIAL_RADIUS)
				.gain(HAMMER_VOLUME)
				.bus(bus),
			mixer,
			listener,
			"firearm-hammer",
		);
	}

	pub fn loop_fizz(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &Mixer,
		listener: &GlobalTransform,
		projectile: Entity,
		world: Vec3,
		velocity: Vec3,
	) {
		commands.entity(projectile).insert((AudioVelocity(velocity), FlightFizz));
		audio.play_loop_or_queue(
			commands,
			&self.fizz,
			clips,
			SpatialEmitter::on(projectile, world)
				.velocity(velocity)
				.radius(FIZZ_SPATIAL_RADIUS)
				.gain(FIZZ_VOLUME)
				.bus(AudioBus::Weapons),
			mixer,
			listener,
			"projectile-fizz",
		);
	}

	pub fn play_impact(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		audio.play_or_queue(
			commands,
			&self.impact,
			clips,
			SpatialOneShot::at(point)
				.radius(IMPACT_SPATIAL_RADIUS)
				.gain(IMPACT_VOLUME)
				.bus(AudioBus::Impacts),
			mixer,
			listener,
			"projectile-impact",
		);
	}
}

impl FromWorld for FirearmFireSounds {
	fn from_world(world: &mut World) -> Self {
		let asset_server = world.resource::<AssetServer>();
		Self::load(asset_server)
	}
}

pub(crate) fn setup_fire_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(FirearmFireSounds::load(&assets));
}

/// Playgrounds that do not spawn a follow-camera listener still get one on the
/// lone `Camera3d`. Skips when a listener already exists or when more than one
/// 3D camera is around.
pub(crate) fn ensure_camera_spatial_listener(
	mut commands: Commands,
	existing: Query<Entity, With<SpatialListener>>,
	cameras: Query<Entity, (With<Camera3d>, Without<SpatialListener>)>,
) {
	if !existing.is_empty() {
		return;
	}
	let mut cameras = cameras.iter();
	let Some(camera) = cameras.next() else {
		return;
	};
	if cameras.next().is_some() {
		return;
	}
	commands.entity(camera).insert(FirearmFireSounds::listener());
}

pub(crate) fn copy_flight_audio_velocity(
	mut flights: Query<(&avian3d::prelude::LinearVelocity, &mut AudioVelocity), With<FlightFizz>>,
) {
	for (linear, mut audio) in &mut flights {
		audio.0 = linear.0;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	fn asset_exists(path: AssetPath) {
		let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(path.as_str());
		assert!(file.is_file(), "{}", file.display());
	}

	#[test]
	fn authored_clips_are_in_assets() {
		asset_exists(WEAPON_FIRE);
		asset_exists(WEAPON_HAMMER);
		asset_exists(WEAPON_FIZZ);
		asset_exists(WEAPON_IMPACT);
	}

	#[test]
	fn authored_spatial_clips_are_mono() {
		for path in [WEAPON_FIRE, WEAPON_HAMMER, WEAPON_FIZZ, WEAPON_IMPACT] {
			let file =
				Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(path.as_str());
			let bytes = match std::fs::read(&file) {
				Ok(bytes) => bytes,
				Err(_) => panic!("{}", file.display()),
			};
			assert!(
				maybraid_audio::decode_wav_mono(&bytes).is_ok(),
				"{} must be authored mono",
				file.display()
			);
		}
	}

	#[test]
	fn each_layer_has_its_own_spatial_radius() {
		assert!((FIRE_SPATIAL_RADIUS - 1.0 / FIRE_SPATIAL_SCALE).abs() < 1e-5);
		assert!((HAMMER_SPATIAL_RADIUS - 1.0 / HAMMER_SPATIAL_SCALE).abs() < 1e-5);
		assert!((FIZZ_SPATIAL_RADIUS - 1.0 / FIZZ_SPATIAL_SCALE).abs() < 1e-5);
		assert!((IMPACT_SPATIAL_RADIUS - 1.0 / IMPACT_SPATIAL_SCALE).abs() < 1e-5);
		assert!(FIRE_SPATIAL_RADIUS > HAMMER_SPATIAL_RADIUS);
		assert!(HAMMER_SPATIAL_RADIUS > IMPACT_SPATIAL_RADIUS);
		assert!(IMPACT_SPATIAL_RADIUS > FIZZ_SPATIAL_RADIUS);
	}

	#[test]
	fn laser_child_stays_on_the_muzzle() {
		for len in [0.02_f32, 4.0, 22.0] {
			let parent_y = 1.0 + len * 0.5;
			let child_world_y = parent_y + LASER_MUZZLE_LOCAL.y * len;
			assert!((child_world_y - 1.0).abs() < 1e-5, "len {len} -> {child_world_y}");
		}
	}

	#[test]
	fn listener_gap_is_a_wide_head() {
		let listener = FirearmFireSounds::listener();
		assert!(
			(listener.right_ear_offset.x - listener.left_ear_offset.x - FIRE_LISTENER_GAP).abs()
				< 1e-5
		);
	}

	#[test]
	fn ensure_listener_lands_on_a_lone_camera() {
		let mut app = App::new();
		app.add_systems(Update, ensure_camera_spatial_listener);
		let camera = app.world_mut().spawn(Camera3d::default()).id();
		app.update();
		assert!(app.world().get::<SpatialListener>(camera).is_some());
	}

	#[test]
	fn ensure_listener_does_not_add_a_second_pair_of_ears() {
		let mut app = App::new();
		app.add_systems(Update, ensure_camera_spatial_listener);
		let camera = app.world_mut().spawn(Camera3d::default()).id();
		let existing = app.world_mut().spawn(FirearmFireSounds::listener()).id();
		app.update();
		assert!(app.world().get::<SpatialListener>(camera).is_none());
		assert!(app.world().get::<SpatialListener>(existing).is_some());
	}
}
