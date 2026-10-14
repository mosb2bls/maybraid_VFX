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

/// Readable beats: salute hold, strafe slide peak, dash lunge.
const POSES: &[IconPose] = &[
	IconPose {
		filename: "healing_salute.png",
		animation: AnimationMode::Jab,
		side: Side::Right,
		progress: 0.36,
		eye: Vec3::new(2.2, 1.5, 2.4),
		look_at: Vec3::new(0.0, 1.0, 0.0),
	},
	IconPose {
		filename: "left_strafe.png",
		animation: AnimationMode::PowerStrafe,
		side: Side::Left,
		progress: 0.48,
		eye: Vec3::new(3.4, 1.15, 0.15),
		look_at: Vec3::new(0.0, 1.0, 0.0),
	},
	IconPose {
		filename: "right_strafe.png",
		animation: AnimationMode::PowerStrafe,
		side: Side::Right,
		progress: 0.48,
		eye: Vec3::new(-3.4, 1.15, 0.15),
		look_at: Vec3::new(0.0, 1.0, 0.0),
	},
	IconPose {
		filename: "forward_dash.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.50,
		eye: Vec3::new(2.7, 0.55, 2.8),
		look_at: Vec3::new(0.0, 0.10, 0.25),
	},
];

const DASH_SIDE_EYE: Vec3 = Vec3::new(3.3, 0.35, 0.40);
const DASH_Q3_EYE: Vec3 = Vec3::new(2.6, 0.50, 2.7);
const DASH_LOOK: Vec3 = Vec3::new(0.0, 0.05, 0.25);

/// Key PowerDash beats: side and three-quarter, full body including feet.
const DASH_POSES: &[IconPose] = &[
	IconPose {
		filename: "power_dash_20_chamber_side.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.20,
		eye: DASH_SIDE_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_20_chamber_3q.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.20,
		eye: DASH_Q3_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_35_stomp_side.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.35,
		eye: DASH_SIDE_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_35_stomp_3q.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.35,
		eye: DASH_Q3_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_50_strike_side.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.50,
		eye: DASH_SIDE_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_50_strike_3q.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.50,
		eye: DASH_Q3_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_65_dash_side.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.65,
		eye: DASH_SIDE_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_65_dash_3q.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 0.65,
		eye: DASH_Q3_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_100_settle_side.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 1.0,
		eye: DASH_SIDE_EYE,
		look_at: DASH_LOOK,
	},
	IconPose {
		filename: "power_dash_100_settle_3q.png",
		animation: AnimationMode::PowerDash,
		side: Side::Right,
		progress: 1.0,
		eye: DASH_Q3_EYE,
		look_at: DASH_LOOK,
	},
];

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct CapturePowerDashPosesArgs {
	/// Directory for raw playground screenshots of PowerDash keyframes.
	#[arg(long)]
	pub out_dir: PathBuf,
}

#[derive(Resource)]
pub struct TanzIconCapture {
	out_dir: PathBuf,
	poses: &'static [IconPose],
	index: usize,
	phase: CapturePhase,
}

#[derive(Clone, Copy)]
enum CapturePhase {
	Pose,
	Wait { until: f32 },
	Shot,
	WaitFile { until: f32 },
}

impl TanzIconCapture {
	pub fn new(out_dir: PathBuf) -> Self {
		Self { out_dir, poses: POSES, index: 0, phase: CapturePhase::Pose }
	}

	pub fn power_dash_poses(out_dir: PathBuf) -> Self {
		Self { out_dir, poses: DASH_POSES, index: 0, phase: CapturePhase::Pose }
	}
}

/// Ground disc at rest-foot height so review shots show planted feet and floor.
fn spawn_review_floor(world: &mut World) {
	let mesh = {
		let mut meshes = world.resource_mut::<Assets<Mesh>>();
		meshes.add(Plane3d::default().mesh().size(16.0, 16.0))
	};
	let material = {
		let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
		materials.add(StandardMaterial {
			base_color: Color::srgb(0.40, 0.36, 0.30),
			perceptual_roughness: 0.92,
			..default()
		})
	};
	world.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_xyz(0.0, -1.52, 0.0)));
}

pub fn start_power_dash_pose_capture(commands: &mut Commands, args: CapturePowerDashPosesArgs) {
	let out_dir = args.out_dir;
	commands.queue(move |world: &mut World| {
		if let Some(mut visible) = world.get_resource_mut::<GameCommandDrawerVisible>() {
			visible.0 = false;
		}
		world.insert_resource(ClearColor(CAPTURE_CLEAR));
		if let Err(error) = std::fs::create_dir_all(&out_dir) {
			error!("power dash pose capture: create {}: {error}", out_dir.display());
		}
		spawn_review_floor(world);
		world.insert_resource(TanzIconCapture::power_dash_poses(out_dir));
	});
}

pub fn start_tanz_icon_capture(commands: &mut Commands, args: CaptureTanzIconsArgs) {
	let out_dir = args.out_dir;
	commands.queue(move |world: &mut World| {
		if let Some(mut visible) = world.get_resource_mut::<GameCommandDrawerVisible>() {
			visible.0 = false;
		}
		world.insert_resource(ClearColor(CAPTURE_CLEAR));
		if let Err(error) = std::fs::create_dir_all(&out_dir) {
			error!("tanz icon capture: create {}: {error}", out_dir.display());
		}
		world.insert_resource(TanzIconCapture::new(out_dir));
	});
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
