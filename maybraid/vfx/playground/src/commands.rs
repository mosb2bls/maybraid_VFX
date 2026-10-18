//! In-game clap commands for the VFX playground.

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};
use maybraid_vfx::canonicalize_effect_name;

pub const PLAYGROUND_CLI_NAME: &str = "vfx";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "vfx",
	version,
	about = "VFX playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Spawn the composed fiery explosion (flash + fireball + smoke + sparks).
	FieryExplosion {
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
		/// World-space Z offset from the default burst origin, in meters.
		#[arg(long, default_value_t = 0.0)]
		distance: f32,
		#[arg(long)]
		seed: Option<u64>,
		/// 1.0 is the authored layer envelope. Higher plays the same ratios faster.
		#[arg(long, default_value_t = 1.0)]
		playback: f32,
	},
	/// Spawn a named definition. Repeat to overlay instances.
	Show {
		#[arg(default_value = "fiery-explosion")]
		effect: String,
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
		#[arg(long, default_value_t = 0.0)]
		distance: f32,
		#[arg(long)]
		seed: Option<u64>,
		/// 1.0 is the authored layer envelope. Higher plays the same ratios faster.
		#[arg(long, default_value_t = 1.0)]
		playback: f32,
	},
	/// Spawn the last effect at near, mid, and far distances.
	Spread,
	/// Orbit the camera around the burst origin.
	Orbit,
	/// Freeze playback once the instance reaches `age` seconds.
	Freeze {
		#[arg(allow_hyphen_values = true)]
		age: f32,
	},
	/// Resume playback after `/freeze`.
	Play,
	/// Stop automatically repeating the last show.
	Noloop,
}

/// Queued `/show` requests. Applied once [`maybraid_vfx::VfxLibrary`] exists.
#[derive(Resource, Default)]
pub struct PendingVfxShows(pub Vec<VfxShowRequest>);

#[derive(Resource, Default)]
pub struct SpreadOnce(pub bool);

#[derive(Clone, Debug)]
pub struct VfxShowRequest {
	pub effect: String,
	pub scale: f32,
	pub intensity: f32,
	pub distance: f32,
	pub seed: Option<u64>,
	pub playback: f32,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct PreviewFreeze {
	pub age: Option<f32>,
}

#[derive(Resource, Clone, Debug)]
pub struct LoopEnabled(pub bool);

impl Default for LoopEnabled {
	fn default() -> Self {
		Self(true)
	}
}

#[derive(Resource, Default, Debug)]
pub struct CameraOrbit {
	pub enabled: bool,
	pub yaw: f32,
	pub radius: f32,
	pub height: f32,
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Help => *console = Self::long_help_string(),
			Self::Script(script) => script.run(commands, console),
			Self::FieryExplosion { scale, intensity, distance, seed, playback } => {
				queue_show(
					commands,
					console,
					"fiery_explosion",
					scale,
					intensity,
					distance,
					seed,
					playback,
				);
			}
			Self::Show { effect, scale, intensity, distance, seed, playback } => {
				let Some(name) = canonicalize_effect_name(&effect) else {
					*console = format!(
						"unknown effect `{effect}` — try fiery-explosion, flash, fireball, smoke, sparks"
					);
					return;
				};
				queue_show(commands, console, name, scale, intensity, distance, seed, playback);
			}
			Self::Spread => {
				*console = "spread near / mid / far".into();
				commands.queue(|world: &mut World| {
					world.resource_mut::<SpreadOnce>().0 = true;
				});
			}
			Self::Orbit => {
				commands.queue(|world: &mut World| {
					let mut orbit = world.resource_mut::<CameraOrbit>();
					orbit.enabled = !orbit.enabled;
					if orbit.radius < 1.0 {
						orbit.radius = 4.2;
						orbit.height = 1.7;
					}
				});
				*console = "orbit toggled".into();
			}
			Self::Freeze { age } => {
				let hold = if age < 0.0 { None } else { Some(age) };
				commands.queue(move |world: &mut World| {
					world.resource_mut::<PreviewFreeze>().age = hold;
					world.resource_mut::<Time<Virtual>>().unpause();
				});
				*console = match hold {
					Some(age) => format!("freeze at {age:.2}s"),
					None => "freeze off".into(),
				};
			}
			Self::Play => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<PreviewFreeze>().age = None;
					world.resource_mut::<Time<Virtual>>().unpause();
				});
				*console = "play".into();
			}
			Self::Noloop => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<LoopEnabled>().0 = false;
				});
				*console = "loop off".into();
			}
		}
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

fn queue_show(
	commands: &mut Commands,
	console: &mut String,
	name: &str,
	scale: f32,
	intensity: f32,
	distance: f32,
	seed: Option<u64>,
	playback: f32,
) {
	let seed_label = seed.map(|s| s.to_string()).unwrap_or_else(|| "auto".into());
	*console = format!(
		"show {name} --scale {scale} --intensity {intensity} --distance {distance} --seed {seed_label} --playback {playback}"
	);
	let name = name.to_string();
	commands.queue(move |world: &mut World| {
		world.resource_mut::<LoopEnabled>().0 = true;
		world.resource_mut::<Time<Virtual>>().unpause();
		world.resource_mut::<PendingVfxShows>().0.push(VfxShowRequest {
			effect: name,
			scale,
			intensity,
			distance,
			seed,
			playback,
		});
	});
}
