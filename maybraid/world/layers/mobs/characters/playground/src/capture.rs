//! Pose a nude default Braidman at each starter Tanz and screenshot it.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_rigs::Side;
use clap::Args;
use game_commands::GameCommandDrawerVisible;

use crate::animation::{AnimationMode, AnimationPlayback};
use crate::camera::{orient_camera, CameraController};
use crate::character::{CharacterConfig, PlaygroundCharacter};

/// Flat chroma used so the silhouette converter can key the character.
pub const CAPTURE_CLEAR: Color = Color::srgb(1.0, 0.0, 1.0);

const FIRST_SETTLE_SECS: f32 = 12.0;
const POSE_SETTLE_SECS: f32 = 2.0;
const SPAWN_TIMEOUT_SECS: f32 = 45.0;
const FILE_TIMEOUT_SECS: f32 = 20.0;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct CaptureTanzIconsArgs {
	/// Directory for raw playground screenshots (`<id>.png`).
	#[arg(long)]
	pub out_dir: PathBuf,
	/// Capture PowerStrafe juke beats instead of the starter Tanz icon set.
	#[arg(long)]
	pub strafe_review: bool,
}

#[derive(Clone, Copy)]
struct IconPose {
	filename: &'static str,
	animation: AnimationMode,
	side: Side,
	/// Mailbox seconds; Tanz clips clamp to `[0, 1]`.
	progress: f32,
	eye: Vec3,
	look_at: Vec3,
}

const ICON_LOOK: Vec3 = Vec3::new(0.0, 1.0, 0.0);
/// Crouched juke lives near the ground plane (rest feet at y = −1.5).
const JUKE_LOOK: Vec3 = Vec3::new(0.0, -0.45, 0.0);
const FRONT_EYE: Vec3 = Vec3::new(0.0, 0.2, 3.5);
const Q3_LEFT_EYE: Vec3 = Vec3::new(-2.3, 0.4, 2.7);
const Q3_RIGHT_EYE: Vec3 = Vec3::new(2.3, 0.4, 2.7);

/// Readable beats: salute hold, strafe slide peak, dash lunge.
const POSES: &[IconPose] = &[
	IconPose {
		filename: "healing_salute.png",
		animation: AnimationMode::Jab,
		side: Side::Right,
		progress: 0.36,
		eye: Vec3::new(2.2, 1.5, 2.4),
		look_at: ICON_LOOK,
	},
	IconPose {
		filename: "left_strafe.png",
		animation: AnimationMode::PowerStrafe,
		side: Side::Left,
		progress: 0.48,
		eye: Q3_LEFT_EYE,
		look_at: JUKE_LOOK,
	},
	IconPose {
		filename: "right_strafe.png",
		animation: AnimationMode::PowerStrafe,
		side: Side::Right,
		progress: 0.48,
		eye: Q3_RIGHT_EYE,
		look_at: JUKE_LOOK,
	},
	IconPose {
		filename: "forward_dash.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.42,
		eye: Vec3::new(2.6, 1.45, 2.8),
		look_at: ICON_LOOK,
	},
];

const fn juke_pose(filename: &'static str, side: Side, progress: f32, eye: Vec3) -> IconPose {
	IconPose {
		filename,
		animation: AnimationMode::PowerStrafe,
		side,
		progress,
		eye,
		look_at: JUKE_LOOK,
	}
}

/// Key juke beats for PR review: front and three-quarter, full body + floor.
const STRAFE_REVIEW_POSES: &[IconPose] = &[
	juke_pose("left_strafe_fake_front.png", Side::Left, 0.10, FRONT_EYE),
	juke_pose("left_strafe_fake_3q.png", Side::Left, 0.10, Q3_LEFT_EYE),
	juke_pose("left_strafe_plant_front.png", Side::Left, 0.20, FRONT_EYE),
	juke_pose("left_strafe_plant_3q.png", Side::Left, 0.20, Q3_LEFT_EYE),
	juke_pose("left_strafe_cut_front.png", Side::Left, 0.35, FRONT_EYE),
	juke_pose("left_strafe_cut_3q.png", Side::Left, 0.35, Q3_LEFT_EYE),
	juke_pose("left_strafe_smear_front.png", Side::Left, 0.50, FRONT_EYE),
	juke_pose("left_strafe_smear_3q.png", Side::Left, 0.50, Q3_LEFT_EYE),
	juke_pose("right_strafe_fake_front.png", Side::Right, 0.10, FRONT_EYE),
	juke_pose("right_strafe_fake_3q.png", Side::Right, 0.10, Q3_RIGHT_EYE),
	juke_pose("right_strafe_smear_front.png", Side::Right, 0.50, FRONT_EYE),
	juke_pose("right_strafe_smear_3q.png", Side::Right, 0.50, Q3_RIGHT_EYE),
];

#[derive(Resource)]
pub struct TanzIconCapture {
	out_dir: PathBuf,
	index: usize,
	phase: CapturePhase,
	poses: &'static [IconPose],
}

#[derive(Clone, Copy)]
enum CapturePhase {
	Pose,
	Wait { until: f32 },
	Shot,
	WaitFile { until: f32 },
}

impl TanzIconCapture {
	fn with_poses(out_dir: PathBuf, poses: &'static [IconPose]) -> Self {
		Self { out_dir, index: 0, phase: CapturePhase::Pose, poses }
	}
}

pub fn start_tanz_icon_capture(commands: &mut Commands, args: CaptureTanzIconsArgs) {
	let out_dir = args.out_dir;
	let review = args.strafe_review;
	let poses = if review { STRAFE_REVIEW_POSES } else { POSES };
	commands.queue(move |world: &mut World| {
		if let Some(mut visible) = world.get_resource_mut::<GameCommandDrawerVisible>() {
			visible.0 = false;
		}
		if review {
			world.insert_resource(ClearColor(Color::srgb(0.82, 0.88, 0.92)));
			spawn_review_ground(world);
		} else {
			world.insert_resource(ClearColor(CAPTURE_CLEAR));
		}
		if let Err(error) = std::fs::create_dir_all(&out_dir) {
			error!("tanz icon capture: create {}: {error}", out_dir.display());
		}
		world.insert_resource(TanzIconCapture::with_poses(out_dir, poses));
	});
}

fn spawn_review_ground(world: &mut World) {
	let mesh = {
		let mut meshes = world.resource_mut::<Assets<Mesh>>();
		meshes.add(Plane3d::default().mesh().size(10.0, 10.0))
	};
	let material = {
		let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
		materials.add(StandardMaterial {
			base_color: Color::srgb(0.46, 0.42, 0.38),
			perceptual_roughness: 0.92,
			..default()
		})
	};
	// Visual mesh feet sit ~0.5 m above the 1 m clip-test shin tail (rest y = −1.5).
	world.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_xyz(0.0, -1.0, 0.0)));
	world.spawn((
		DirectionalLight { illuminance: 4000.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.15, 0.0)),
	));
}

pub fn drive_tanz_icon_capture(
	mut commands: Commands,
	time: Res<Time>,
	capturing: Query<(), With<Capturing>>,
	roots: Query<Entity, With<PlaygroundCharacter>>,
	mut capture: ResMut<TanzIconCapture>,
	mut exit: MessageWriter<AppExit>,
) {
	if capture.index >= capture.poses.len() {
		info!("tanz icon capture: finished {}", capture.out_dir.display());
		exit.write(AppExit::Success);
		return;
	}
	let pose = capture.poses[capture.index];
	let path = capture.out_dir.join(pose.filename);
	let now = time.elapsed_secs();
	match capture.phase {
		CapturePhase::Pose => {
			apply_pose(&mut commands, pose);
			let settle = if capture.index == 0 { FIRST_SETTLE_SECS } else { POSE_SETTLE_SECS };
			capture.phase = CapturePhase::Wait { until: now + settle };
			info!("tanz icon capture: pose {} progress={:.2}", pose.filename, pose.progress);
		}
		CapturePhase::Wait { until } => {
			if roots.is_empty() {
				if now > until + SPAWN_TIMEOUT_SECS {
					error!("tanz icon capture: character never spawned");
					exit.write(AppExit::from_code(1));
				}
				return;
			}
			if now < until {
				return;
			}
			capture.phase = CapturePhase::Shot;
		}
		CapturePhase::Shot => {
			if !capturing.is_empty() {
				return;
			}
			commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
			capture.phase = CapturePhase::WaitFile { until: now + FILE_TIMEOUT_SECS };
			info!("tanz icon capture: screenshot {}", path.display());
		}
		CapturePhase::WaitFile { until } => {
			if path.is_file() && capturing.is_empty() {
				capture.index += 1;
				capture.phase = CapturePhase::Pose;
				return;
			}
			if now > until {
				error!("tanz icon capture: timed out waiting for {}", path.display());
				exit.write(AppExit::from_code(1));
			}
		}
	}
}

fn apply_pose(commands: &mut Commands, pose: IconPose) {
	commands.queue(move |world: &mut World| {
		{
			let mut config = world.resource_mut::<CharacterConfig>();
			config.animation = pose.animation;
			config.side = pose.side;
		}
		{
			let mut playback = world.resource_mut::<AnimationPlayback>();
			playback.paused = true;
			playback.looping = false;
			playback.speed = 1.0;
			playback.elapsed = pose.progress;
			playback.scrub = Some(pose.progress);
			playback.joint.clear();
		}
		let mut cameras = world.query::<(&mut Transform, &mut CameraController)>();
		if let Some((mut transform, mut controller)) = cameras.iter_mut(world).next() {
			orient_camera(&mut transform, &mut controller, pose.eye, pose.look_at);
		}
	});
}
