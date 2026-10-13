//! `/character` subcommands for the animation lab.

use bevy::prelude::*;
use character_rigs::Side;
use clap::{Args, Subcommand, ValueEnum};

use crate::animation::{AnimationMode, AnimationPlayback};
use crate::camera::{orient_camera, CameraController};
use crate::character::{request_dump_bones, CharacterConfig, CharacterSpecies};

#[derive(Clone, Subcommand)]
pub enum Character {
	/// Spawn a clothed species through the runtime assembly path.
	Assemble(AssembleArgs),
	/// Switch the sampled clip without respawning the character.
	Animate(AnimateArgs),
	/// Pause, scrub, or change the speed of the mailbox clock.
	Playback(PlaybackArgs),
	/// Frame the character from the front, the side, or three-quarter.
	Camera(CameraArgs),
	/// Show bone-local, parent-local, and character-space axes for one bone.
	Joint(JointArgs),
	/// Print the live rig bone hierarchy to the HUD console.
	DumpBones,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct AssembleArgs {
	/// Species recipe (`braidman` is the humanoid lab subject).
	#[arg(long, value_enum, default_value_t = CharacterSpecies::Braidman)]
	pub species: CharacterSpecies,

	/// Procedural clip (`still`, `walk`, `run`, `jab`, `leap`, …).
	#[arg(long, value_enum, default_value_t = AnimationMode::Run)]
	pub animation: AnimationMode,

	/// Lead side for sided gestures (`jab`). Ignored by symmetric clips.
	#[arg(long, value_enum, default_value_t = GestureSide::Right)]
	pub side: GestureSide,

	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub translate: Vec3,

	/// Scale factors `x,y,z`.
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub scale: Vec3,

	/// Euler rotation in degrees around X, then Y, then Z.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub rotate_euler: Vec3,
}

impl Character {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Character::Assemble(args) => {
				let config = args.into_character_config();
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<CharacterConfig>() = config;
					reset_clip_clock(&mut world.resource_mut::<AnimationPlayback>());
				});
			}
			Character::Animate(args) => {
				let animation = args.animation;
				let side = args.side.into_side();
				commands.queue(move |world: &mut World| {
					let mut config = world.resource_mut::<CharacterConfig>();
					config.animation = animation;
					config.side = side;
					reset_clip_clock(&mut world.resource_mut::<AnimationPlayback>());
				});
			}
			Character::Playback(args) => {
				commands.queue(move |world: &mut World| {
					let mut playback = world.resource_mut::<AnimationPlayback>();
					if args.pause {
						playback.paused = true;
					}
					if args.resume {
						playback.paused = false;
					}
					if let Some(speed) = args.speed {
						playback.speed = speed;
					}
					if let Some(progress) = args.progress {
						playback.scrub = Some(progress);
					}
					if let Some(phase) = args.phase {
						playback.phase = phase;
					}
					if let Some(scale) = args.leg_scale {
						playback.leg_scale = scale;
					}
					if args.once {
						playback.looping = false;
					}
					if args.looping {
						playback.looping = true;
					}
				});
			}
			Character::Camera(args) => {
				let eye = match args.view {
					CameraView::Front => Vec3::new(0.0, 1.2, 3.2),
					CameraView::Side => Vec3::new(3.2, 1.2, 0.0),
					CameraView::ThreeQuarter => Vec3::new(2.2, 1.5, 2.4),
				};
				commands.queue(move |world: &mut World| {
					let mut cameras = world.query::<(&mut Transform, &mut CameraController)>();
					if let Some((mut transform, mut controller)) = cameras.iter_mut(world).next() {
						orient_camera(
							&mut transform,
							&mut controller,
							eye,
							Vec3::new(0.0, 1.0, 0.0),
						);
					}
				});
			}
			Character::Joint(args) => {
				let name = args.name;
				let show_rest = !args.hide_rest;
				let clear = args.clear;
				let flexion = args.flexion;
				let lateral = args.lateral;
				let axial = args.axial;
				commands.queue(move |world: &mut World| {
					let mut playback = world.resource_mut::<AnimationPlayback>();
					playback.joint = name;
					playback.show_rest = show_rest;
					if clear {
						playback.joint_degrees = None;
					}
					if flexion.is_some() || lateral.is_some() || axial.is_some() {
						let mut degrees =
							playback.joint_degrees.unwrap_or(crate::animation::JointDegrees {
								flexion: 0.0,
								lateral: 0.0,
								axial: 0.0,
							});
						if let Some(flexion) = flexion {
							degrees.flexion = flexion;
						}
						if let Some(lateral) = lateral {
							degrees.lateral = lateral;
						}
						if let Some(axial) = axial {
							degrees.axial = axial;
						}
						playback.joint_degrees = Some(degrees);
					}
				});
			}
			Character::DumpBones => request_dump_bones(commands),
		}
	}
}

impl AssembleArgs {
	fn into_character_config(self) -> CharacterConfig {
		let rot = Quat::from_euler(
			EulerRot::XYZ,
			self.rotate_euler.x.to_radians(),
			self.rotate_euler.y.to_radians(),
			self.rotate_euler.z.to_radians(),
		);
		CharacterConfig {
			species: self.species,
			animation: self.animation,
			side: self.side.into_side(),
			transform: Transform::from_translation(self.translate)
				.with_rotation(rot)
				.with_scale(self.scale),
		}
	}
}

fn reset_clip_clock(playback: &mut AnimationPlayback) {
	playback.elapsed = 0.0;
	playback.scrub = None;
	playback.paused = false;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum GestureSide {
	Left,
	#[default]
	Right,
}

impl GestureSide {
	fn into_side(self) -> Side {
		match self {
			Self::Left => Side::Left,
			Self::Right => Side::Right,
		}
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct AnimateArgs {
	/// Clip to sample (`still`, `walk`, `run`, `jab`, `leap`, `soaring`, …).
	#[arg(value_enum)]
	pub animation: AnimationMode,

	/// Lead side for sided gestures (`jab`).
	#[arg(long, value_enum, default_value_t = GestureSide::Right)]
	pub side: GestureSide,
}

fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).collect();
	if parts.len() != 3 {
		return Err(format!("expected x,y,z, got {s:?}"));
	}
	let x = parts[0].parse::<f32>().map_err(|e| e.to_string())?;
	let y = parts[1].parse::<f32>().map_err(|e| e.to_string())?;
	let z = parts[2].parse::<f32>().map_err(|e| e.to_string())?;
	Ok(Vec3::new(x, y, z))
}

#[derive(Clone, Args)]
pub struct PlaybackArgs {
	/// Freeze the sample clock.
	#[arg(long)]
	pub pause: bool,
	/// Resume the sample clock.
	#[arg(long)]
	pub resume: bool,
	/// Scale applied to delta time.
	#[arg(long)]
	pub speed: Option<f32>,
	/// Jump the sample clock to this many seconds.
	#[arg(long)]
	pub progress: Option<f32>,
	/// Add this many seconds before sampling. `0.5` is the opposite gait phase.
	#[arg(long)]
	pub phase: Option<f32>,
	/// Scale femur and shin rest length. `0.75` is short, `1.25` is long.
	#[arg(long)]
	pub leg_scale: Option<f32>,
	/// Stop the clock at one second.
	#[arg(long)]
	pub once: bool,
	/// Keep the clock running.
	#[arg(long)]
	pub looping: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CameraView {
	Front,
	Side,
	#[value(name = "three-quarter")]
	ThreeQuarter,
}

#[derive(Clone, Args)]
pub struct CameraArgs {
	#[arg(value_enum)]
	pub view: CameraView,
}

#[derive(Clone, Args)]
pub struct JointArgs {
	/// Animation bone, for example `femur.L` or `root`. Empty clears the gizmo.
	pub name: String,
	/// Hide the effective-rest direction marker.
	#[arg(long)]
	pub hide_rest: bool,
	/// Anatomical forward bend, in degrees. Positive tips a +Y bone toward +Z.
	#[arg(long)]
	pub flexion: Option<f32>,
	/// Anatomical side bend, in degrees.
	#[arg(long)]
	pub lateral: Option<f32>,
	/// Anatomical axial turn, in degrees.
	#[arg(long)]
	pub axial: Option<f32>,
	/// Return the bone to the clip pose.
	#[arg(long)]
	pub clear: bool,
}
