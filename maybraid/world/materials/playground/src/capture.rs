//! Env-gated column POM evidence capture for CI / agent hosts.
//!
//! Set `MAYBRAID_CAPTURE_DIR` to a writable directory. The playground hides
//! the command drawer, stamps `doric_column` on the contract cylinder, walks
//! a raking-light A/B (POM on vs flat), a silhouette+cavity pose, and a
//! tapered 0.8 D neck, writes PNGs, and exits.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};
use building_shaders::{doric_column_flat_ref, doric_column_ref};
use game_commands::ui::{DebugHudRoot, GameCommandDrawerVisible};

use crate::camera::CameraController;
use crate::preview::PreviewConfig;
use crate::shape::PreviewShape;

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
	/// Low key, roughly perpendicular to the view so flute normals read.
	Rake,
	Day,
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
	pub shape: PreviewShape,
}

/// Shaft sits on the preview root (y = 1): compact cylinder mid ≈ 2.26,
/// tapered 4.5 D mid ≈ 2.89.
pub const COLUMN_CAPTURE_SHOTS: &[CaptureShot] = &[
	CaptureShot {
		name: "rake-grazing-pom-on",
		camera: Vec3::new(1.42, 2.32, 1.08),
		look_at: Vec3::new(0.12, 2.26, 0.04),
		light: CaptureLight::Rake,
		look: CaptureLook::Pom,
		shape: PreviewShape::Cylinder,
	},
	CaptureShot {
		name: "rake-grazing-pom-off",
		camera: Vec3::new(1.42, 2.32, 1.08),
		look_at: Vec3::new(0.12, 2.26, 0.04),
		light: CaptureLight::Rake,
		look: CaptureLook::Flat,
		shape: PreviewShape::Cylinder,
	},
	CaptureShot {
		name: "rake-silhouette-cavities",
		camera: Vec3::new(1.85, 2.55, 2.45),
		look_at: Vec3::new(0.0, 2.26, 0.0),
		light: CaptureLight::Rake,
		look: CaptureLook::Pom,
		shape: PreviewShape::Cylinder,
	},
	CaptureShot {
		name: "rake-taper-pom-on",
		camera: Vec3::new(1.55, 2.95, 1.25),
		look_at: Vec3::new(0.10, 2.89, 0.04),
		light: CaptureLight::Rake,
		look: CaptureLook::Pom,
		shape: PreviewShape::Taper,
	},
	CaptureShot {
		name: "rake-taper-silhouette",
		camera: Vec3::new(2.15, 3.15, 2.85),
		look_at: Vec3::new(0.0, 2.89, 0.0),
		light: CaptureLight::Rake,
		look: CaptureLook::Pom,
		shape: PreviewShape::Taper,
	},
	CaptureShot {
		name: "day-head-on-close",
		camera: Vec3::new(0.0, 2.40, 4.2),
		look_at: Vec3::new(0.0, 2.26, 0.0),
		light: CaptureLight::Day,
		look: CaptureLook::Pom,
		shape: PreviewShape::Cylinder,
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

/// Collapse the command drawer so it does not cover the shaft.
pub(crate) fn hide_capture_hud(
	mut visible: ResMut<GameCommandDrawerVisible>,
	mut roots: Query<&mut Visibility, With<DebugHudRoot>>,
) {
	visible.0 = false;
	for mut root in &mut roots {
		*root = Visibility::Hidden;
	}
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
	preview: ResMut<'w, PreviewConfig>,
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
		self.preview.material = match shot.look {
			CaptureLook::Pom => doric_column_ref(),
			CaptureLook::Flat => doric_column_flat_ref(),
		};
		self.preview.shape = shot.shape;

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
		}
	}

	fn apply_rake(&mut self, shot: &CaptureShot) {
		let view = (shot.look_at - shot.camera).normalize_or_zero();
		let side = view.cross(Vec3::Y).normalize_or_zero();
		let from = shot.look_at + side * 4.2 + Vec3::Y * 0.28 - view * 0.4;
		for (mut transform, mut light) in self.keys.iter_mut() {
			*transform = Transform::from_translation(from).looking_at(shot.look_at, Vec3::Y);
			light.illuminance = 16000.0;
			light.color = Color::srgb(1.0, 0.93, 0.78);
		}
		for mut light in self.fills.iter_mut() {
			light.illuminance = 1100.0;
			light.color = Color::srgb(0.45, 0.55, 0.78);
		}
		self.ambient.brightness = 70.0;
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
		self.clear.0 = Color::srgb(0.08, 0.09, 0.11);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capture_set_has_rake_ab_and_silhouette() -> Result<(), String> {
		assert_eq!(COLUMN_CAPTURE_SHOTS.len(), 6);
		let on = COLUMN_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "rake-grazing-pom-on")
			.ok_or("missing pom-on")?;
		let off = COLUMN_CAPTURE_SHOTS
			.iter()
			.find(|shot| shot.name == "rake-grazing-pom-off")
			.ok_or("missing pom-off")?;
		assert_eq!(on.camera, off.camera);
		assert_eq!(on.look_at, off.look_at);
		assert_eq!(on.light, CaptureLight::Rake);
		assert_eq!(off.light, CaptureLight::Rake);
		assert_eq!(on.look, CaptureLook::Pom);
		assert_eq!(off.look, CaptureLook::Flat);
		assert_eq!(on.shape, PreviewShape::Cylinder);
		assert!(COLUMN_CAPTURE_SHOTS.iter().any(|shot| shot.name.contains("silhouette")));
		assert!(
			COLUMN_CAPTURE_SHOTS
				.iter()
				.any(|shot| { shot.name.contains("taper") && shot.shape == PreviewShape::Taper })
		);
		Ok(())
	}
}
