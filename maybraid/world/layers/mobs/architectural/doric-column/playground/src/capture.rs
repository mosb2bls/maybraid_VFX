//! Env-gated kit column POM evidence (`MAYBRAID_CAPTURE_DIR`).
//!
//! Spawns a default [`DoricColumnSpec`] encounter column (kit placeholders + shaft
//! `doric-column-shaft-pom`), walks day / raking / night lighting with POM on vs off,
//! writes PNGs, and exits.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};
use building_shaders::{doric_column_kit_shaft_flat_ref, doric_column_kit_shaft_ref};
use doric_column::{spawn_doric_column_encounter, DoricColumnRoot, DoricColumnSpec};

use crate::camera::CameraController;
use crate::preview::CaptureKitColumn;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
const WARMUP_FRAMES: u32 = 120;
const SETTLE_FRAMES: u32 = 18;
const SAVE_FRAMES: u32 = 28;

#[derive(Component)]
pub struct KeySun;

#[derive(Component)]
pub struct FillSky;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureLight {
	Rake,
	Day,
	Night,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureLook {
	Pom,
	Flat,
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
	pub light: CaptureLight,
	pub look: CaptureLook,
}

/// Default encounter column: D = 1.5 m, shaft mid ≈ 4.8 m.
const SHAFT_MID_Y: f32 = 4.8;

pub const KIT_COLUMN_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot {
		name: "kit-day-pom-on",
		camera: Vec3::new(0.0, 5.4, 11.5),
		look_at: Vec3::new(0.0, SHAFT_MID_Y, 0.0),
		light: CaptureLight::Day,
		look: CaptureLook::Pom,
	},
	CaptureShot {
		name: "kit-day-pom-off",
		camera: Vec3::new(0.0, 5.4, 11.5),
		look_at: Vec3::new(0.0, SHAFT_MID_Y, 0.0),
		light: CaptureLight::Day,
		look: CaptureLook::Flat,
	},
	CaptureShot {
		name: "kit-rake-pom-on",
		camera: Vec3::new(2.35, 5.05, 1.85),
		look_at: Vec3::new(0.05, SHAFT_MID_Y, 0.0),
		light: CaptureLight::Rake,
		look: CaptureLook::Pom,
	},
	CaptureShot {
		name: "kit-rake-pom-off",
		camera: Vec3::new(2.35, 5.05, 1.85),
		look_at: Vec3::new(0.05, SHAFT_MID_Y, 0.0),
		light: CaptureLight::Rake,
		look: CaptureLook::Flat,
	},
	CaptureShot {
		name: "kit-night-pom-on",
		camera: Vec3::new(-3.2, 5.8, 7.5),
		look_at: Vec3::new(0.0, SHAFT_MID_Y, 0.0),
		light: CaptureLight::Night,
		look: CaptureLook::Pom,
	},
	CaptureShot {
		name: "kit-night-pom-off",
		camera: Vec3::new(-3.2, 5.8, 7.5),
		look_at: Vec3::new(0.0, SHAFT_MID_Y, 0.0),
		light: CaptureLight::Night,
		look: CaptureLook::Flat,
	},
];

#[derive(Resource, Debug)]
pub struct CaptureSession {
	pub dir: PathBuf,
	phase: CapturePhase,
	look: CaptureLook,
}

#[derive(Clone, Copy, Debug)]
enum CapturePhase {
	Warmup { frames: u32 },
	Settle { index: usize, frames: u32 },
	Snap { index: usize },
	WaitSave { index: usize, frames: u32 },
}

impl CaptureSession {
	pub fn from_env() -> Option<Self> {
		let dir = std::env::var_os(CAPTURE_DIR_ENV)?;
		if dir.is_empty() {
			return None;
		}
		Some(Self {
			dir: PathBuf::from(dir),
			phase: CapturePhase::Warmup { frames: 0 },
			look: CaptureLook::Pom,
		})
	}
}

fn spec_for_look(look: CaptureLook) -> DoricColumnSpec {
	let mut spec = DoricColumnSpec::default();
	spec.materials.shaft = match look {
		CaptureLook::Pom => doric_column_kit_shaft_ref(),
		CaptureLook::Flat => doric_column_kit_shaft_flat_ref(),
	};
	spec
}

pub fn spawn_capture_column(commands: &mut Commands, look: CaptureLook) {
	let spec = spec_for_look(look);
	spawn_doric_column_encounter(commands, &spec, Transform::IDENTITY, false);
	commands.spawn((CaptureKitColumn, Name::new("kit-capture-column")));
}

#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(crate) struct CaptureView<'w, 's> {
	cameras: Query<
		'w,
		's,
		(&'static mut Transform, &'static mut CameraController),
		(With<Camera3d>, Without<KeySun>, Without<FillSky>),
	>,
	keys: Query<
		'w,
		's,
		(&'static mut Transform, &'static mut DirectionalLight),
		(With<KeySun>, Without<Camera3d>, Without<FillSky>),
	>,
	fills: Query<
		'w,
		's,
		&'static mut DirectionalLight,
		(With<FillSky>, Without<KeySun>, Without<Camera3d>),
	>,
	ambient: ResMut<'w, GlobalAmbientLight>,
	clear: ResMut<'w, ClearColor>,
}

pub(crate) fn tick_capture(
	mut commands: Commands,
	mut session: ResMut<CaptureSession>,
	mut view: CaptureView,
	capturing: Query<Entity, With<Capturing>>,
	columns: Query<Entity, With<DoricColumnRoot>>,
	mut exit: MessageWriter<AppExit>,
) {
	match session.phase {
		CapturePhase::Warmup { frames } => {
			if frames == 0 {
				view.apply_shot(&KIT_COLUMN_CAPTURE_SHOTS[0]);
				spawn_capture_column(&mut commands, KIT_COLUMN_CAPTURE_SHOTS[0].look);
				session.look = KIT_COLUMN_CAPTURE_SHOTS[0].look;
			}
			if frames + 1 >= WARMUP_FRAMES {
				session.phase = CapturePhase::Settle { index: 0, frames: 0 };
			} else {
				session.phase = CapturePhase::Warmup { frames: frames + 1 };
			}
		}
		CapturePhase::Settle { index, frames } => {
			if frames == 0 {
				if let Some(shot) = KIT_COLUMN_CAPTURE_SHOTS.get(index) {
					if session.look != shot.look {
						for entity in &columns {
							commands.entity(entity).despawn();
						}
						spawn_capture_column(&mut commands, shot.look);
						session.look = shot.look;
					}
					view.apply_shot(shot);
				}
			}
			if frames + 1 >= SETTLE_FRAMES {
				session.phase = CapturePhase::Snap { index };
			} else {
				session.phase = CapturePhase::Settle { index, frames: frames + 1 };
			}
		}
		CapturePhase::Snap { index } => {
			let Some(shot) = KIT_COLUMN_CAPTURE_SHOTS.get(index) else {
				exit.write(AppExit::Success);
				return;
			};
			if let Err(error) = std::fs::create_dir_all(&session.dir) {
				error!("kit column capture: cannot create {}: {error}", session.dir.display());
				exit.write(AppExit::from_code(2));
				return;
			}
			let path = session.dir.join(format!("{}.png", shot.name));
			info!("kit column capture: {}", path.display());
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
			session.phase = CapturePhase::WaitSave { index, frames: 0 };
		}
		CapturePhase::WaitSave { index, frames } => {
			if (capturing.iter().next().is_none() && frames >= 2) || frames + 1 >= SAVE_FRAMES {
				let next = index + 1;
				if next >= KIT_COLUMN_CAPTURE_SHOTS.len() {
					exit.write(AppExit::Success);
				} else {
					session.phase = CapturePhase::Settle { index: next, frames: 0 };
				}
			} else {
				session.phase = CapturePhase::WaitSave { index, frames: frames + 1 };
			}
		}
	}
}

impl CaptureView<'_, '_> {
	fn apply_shot(&mut self, shot: &CaptureShot) {
		if let Ok((mut transform, mut controller)) = self.cameras.single_mut() {
			*transform = Transform::from_translation(shot.camera).looking_at(shot.look_at, Vec3::Y);
			let rotation = transform.rotation;
			let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
			let sin_yaw = 2.0 * (w * y + x * z);
			let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
			controller.yaw = sin_yaw.atan2(cos_yaw);
			let sin_pitch = 2.0 * (w * x - y * z);
			controller.pitch = sin_pitch.asin();
		}

		match shot.light {
			CaptureLight::Rake => self.apply_rake(shot),
			CaptureLight::Day => self.apply_day(),
			CaptureLight::Night => self.apply_night(),
		}
	}

	fn apply_rake(&mut self, shot: &CaptureShot) {
		let view = (shot.look_at - shot.camera).normalize_or_zero();
		let side = view.cross(Vec3::Y).normalize_or_zero();
		let from = shot.look_at + side * 5.5 + Vec3::Y * 0.35 - view * 0.5;
		for (mut transform, mut light) in self.keys.iter_mut() {
			*transform = Transform::from_translation(from).looking_at(shot.look_at, Vec3::Y);
			light.illuminance = 16500.0;
			light.color = Color::srgb(1.0, 0.93, 0.78);
		}
		for mut light in self.fills.iter_mut() {
			light.illuminance = 1200.0;
			light.color = Color::srgb(0.45, 0.55, 0.78);
		}
		self.ambient.brightness = 75.0;
		self.ambient.color = Color::srgb(0.35, 0.40, 0.52);
		self.clear.0 = Color::srgb(0.07, 0.08, 0.10);
	}

	fn apply_day(&mut self) {
		use std::f32::consts::PI;
		for (mut transform, mut light) in self.keys.iter_mut() {
			*transform =
				Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0));
			light.illuminance = 9800.0;
			light.color = Color::srgb(1.0, 0.92, 0.82);
		}
		for mut light in self.fills.iter_mut() {
			light.illuminance = 2400.0;
			light.color = Color::srgb(0.55, 0.68, 1.0);
		}
		self.ambient.brightness = 220.0;
		self.ambient.color = Color::srgb(0.72, 0.78, 0.92);
		self.clear.0 = Color::srgb(0.52, 0.58, 0.64);
	}

	fn apply_night(&mut self) {
		use std::f32::consts::PI;
		for (mut transform, mut light) in self.keys.iter_mut() {
			*transform =
				Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, -PI / 3.0, 0.0));
			light.illuminance = 4200.0;
			light.color = Color::srgb(0.72, 0.78, 0.95);
		}
		for mut light in self.fills.iter_mut() {
			light.illuminance = 180.0;
			light.color = Color::srgb(0.25, 0.30, 0.42);
		}
		self.ambient.brightness = 28.0;
		self.ambient.color = Color::srgb(0.18, 0.22, 0.32);
		self.clear.0 = Color::srgb(0.03, 0.04, 0.07);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn kit_capture_set_has_lighting_ab_pairs() -> Result<(), String> {
		assert_eq!(KIT_COLUMN_CAPTURE_SHOTS.len(), 6);
		for light in [CaptureLight::Day, CaptureLight::Rake, CaptureLight::Night] {
			let on = KIT_COLUMN_CAPTURE_SHOTS
				.iter()
				.find(|shot| shot.light == light && shot.look == CaptureLook::Pom)
				.ok_or_else(|| format!("missing pom for {light:?}"))?;
			let off = KIT_COLUMN_CAPTURE_SHOTS
				.iter()
				.find(|shot| shot.light == light && shot.look == CaptureLook::Flat)
				.ok_or_else(|| format!("missing flat for {light:?}"))?;
			assert_eq!(on.camera, off.camera);
			assert_eq!(on.look_at, off.look_at);
		}
		Ok(())
	}
}
