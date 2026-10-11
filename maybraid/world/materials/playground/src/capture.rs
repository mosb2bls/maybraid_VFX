//! Env-gated column POM evidence capture for CI / agent hosts.
//!
//! Set `MAYBRAID_CAPTURE_DIR` to a writable directory. The playground stamps
//! `doric_column` on the contract cylinder, walks head-on / grazing poses at
//! a couple of distances (day and night), writes PNGs, and exits.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use building_shaders::doric_column_ref;

use crate::camera::CameraController;
use crate::preview::PreviewConfig;
use crate::shape::PreviewShape;

pub const CAPTURE_DIR_ENV: &str = "MAYBRAID_CAPTURE_DIR";
const WARMUP_FRAMES: u32 = 120;
const SETTLE_FRAMES: u32 = 16;
const SAVE_FRAMES: u32 = 28;

#[derive(Component)]
pub struct KeySun;

#[derive(Component)]
pub struct FillSky;

#[derive(Clone, Copy, Debug)]
pub struct CaptureShot {
	pub name: &'static str,
	pub camera: Vec3,
	pub look_at: Vec3,
	pub night: bool,
}

/// Head-on and grazing, close and farther, plus a night pair.
pub const COLUMN_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot {
		name: "day-head-on-close",
		camera: Vec3::new(0.0, 1.15, 3.6),
		look_at: Vec3::new(0.0, 1.0, 0.0),
		night: false,
	},
	CaptureShot {
		name: "day-grazing-close",
		camera: Vec3::new(1.55, 1.12, 1.15),
		look_at: Vec3::new(0.2, 1.0, 0.1),
		night: false,
	},
	CaptureShot {
		name: "day-head-on-mid",
		camera: Vec3::new(0.15, 1.45, 7.8),
		look_at: Vec3::new(0.0, 1.0, 0.0),
		night: false,
	},
	CaptureShot {
		name: "day-grazing-far",
		camera: Vec3::new(11.5, 1.8, 3.4),
		look_at: Vec3::new(0.15, 1.05, 0.0),
		night: false,
	},
	CaptureShot {
		name: "night-head-on-close",
		camera: Vec3::new(0.0, 1.15, 3.6),
		look_at: Vec3::new(0.0, 1.0, 0.0),
		night: true,
	},
	CaptureShot {
		name: "night-grazing-close",
		camera: Vec3::new(1.55, 1.12, 1.15),
		look_at: Vec3::new(0.2, 1.0, 0.1),
		night: true,
	},
];

#[derive(Resource, Debug)]
pub struct CaptureSession {
	pub dir: PathBuf,
	phase: CapturePhase,
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
		Some(Self { dir: PathBuf::from(dir), phase: CapturePhase::Warmup { frames: 0 } })
	}
}

pub(crate) fn apply_column_preview(mut config: ResMut<PreviewConfig>) {
	config.material = doric_column_ref();
	config.shape = PreviewShape::Cylinder;
}

#[derive(SystemParam)]
pub(crate) struct CaptureView<'w, 's> {
	cameras: Query<'w, 's, (&'static mut Transform, &'static mut CameraController), With<Camera3d>>,
	keys: Query<'w, 's, &'static mut DirectionalLight, With<KeySun>>,
	fills: Query<'w, 's, &'static mut DirectionalLight, (With<FillSky>, Without<KeySun>)>,
	ambient: ResMut<'w, GlobalAmbientLight>,
	clear: ResMut<'w, ClearColor>,
}

pub(crate) fn tick_capture(
	mut commands: Commands,
	mut session: ResMut<CaptureSession>,
	mut view: CaptureView,
	capturing: Query<Entity, With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
) {
	match session.phase {
		CapturePhase::Warmup { frames } => {
			if frames == 0 {
				view.apply_shot(&COLUMN_CAPTURE_SHOTS[0]);
			}
			if frames + 1 >= WARMUP_FRAMES {
				session.phase = CapturePhase::Settle { index: 0, frames: 0 };
			} else {
				session.phase = CapturePhase::Warmup { frames: frames + 1 };
			}
		}
		CapturePhase::Settle { index, frames } => {
			if frames == 0 {
				if let Some(shot) = COLUMN_CAPTURE_SHOTS.get(index) {
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
			let Some(shot) = COLUMN_CAPTURE_SHOTS.get(index) else {
				exit.write(AppExit::Success);
				return;
			};
			if let Err(error) = std::fs::create_dir_all(&session.dir) {
				error!("column capture: cannot create {}: {error}", session.dir.display());
				exit.write(AppExit::from_code(2));
				return;
			}
			let path = session.dir.join(format!("{}.png", shot.name));
			info!("column capture: {}", path.display());
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
			session.phase = CapturePhase::WaitSave { index, frames: 0 };
		}
		CapturePhase::WaitSave { index, frames } => {
			if (capturing.iter().next().is_none() && frames >= 2) || frames + 1 >= SAVE_FRAMES {
				let next = index + 1;
				if next >= COLUMN_CAPTURE_SHOTS.len() {
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
		if shot.night {
			for mut light in self.keys.iter_mut() {
				light.illuminance = 1400.0;
				light.color = Color::srgb(0.48, 0.58, 0.92);
			}
			for mut light in self.fills.iter_mut() {
				light.illuminance = 520.0;
				light.color = Color::srgb(0.32, 0.40, 0.70);
			}
			self.ambient.brightness = 70.0;
			self.ambient.color = Color::srgb(0.28, 0.34, 0.55);
			self.clear.0 = Color::srgb(0.02, 0.03, 0.055);
		} else {
			for mut light in self.keys.iter_mut() {
				light.illuminance = 9800.0;
				light.color = Color::srgb(1.0, 0.92, 0.82);
			}
			for mut light in self.fills.iter_mut() {
				light.illuminance = 2400.0;
				light.color = Color::srgb(0.55, 0.68, 1.0);
			}
			self.ambient.brightness = 220.0;
			self.ambient.color = Color::srgb(0.72, 0.78, 0.92);
			self.clear.0 = Color::srgb(0.08, 0.09, 0.11);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capture_set_covers_angles_distances_and_night() -> Result<(), String> {
		assert_eq!(COLUMN_CAPTURE_SHOTS.len(), 6);
		assert!(COLUMN_CAPTURE_SHOTS.iter().any(|shot| shot.name.contains("head-on")));
		assert!(COLUMN_CAPTURE_SHOTS.iter().any(|shot| shot.name.contains("grazing")));
		assert!(COLUMN_CAPTURE_SHOTS.iter().any(|shot| shot.name.contains("close")));
		assert!(COLUMN_CAPTURE_SHOTS
			.iter()
			.any(|shot| shot.name.contains("far") || shot.name.contains("mid")));
		assert!(COLUMN_CAPTURE_SHOTS.iter().any(|shot| shot.night));
		assert!(COLUMN_CAPTURE_SHOTS.iter().any(|shot| !shot.night));
		Ok(())
	}
}
