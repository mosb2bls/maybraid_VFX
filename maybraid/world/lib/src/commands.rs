//! Slim world-playground commands. Forest + terrain extents are baked in.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use world_player::commands::{RequestMeshStats, RequestModeCharacter, RequestModeFree};
use world_player::{CharacterSpecies, RequestFpsToggle, RequestSetCharacter};

use clap::{Parser, Subcommand};
use game_commands::command::{CommandConsoleOutput, CommandScript, GameCommand};

use maybraid_sky::{
	SkyClock, SkyCommand, SKY_PHASE_DAWN, SKY_PHASE_DUSK, SKY_PHASE_GOLDEN, SKY_PHASE_MORNING,
	SKY_PHASE_NIGHT, SKY_PHASE_NOON,
};

use mob_scenes::MobKind;

use crate::mob_spawn::RequestSpawnMobs;
use crate::{RequestForceOffscreen, RequestRenderScale, RequestVsyncToggle};

pub const PLAYGROUND_CLI_NAME: &str = "maybraid-world";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "maybraid-world",
	version,
	about = "World model: Durham terrain, streamed forest, sky dome, character. --start-at X,Z (or MAYBRAID_START_AT) places the player on that XZ.",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Switch between free-look fly camera and third-person character control.
	#[command(subcommand)]
	Mode(Mode),
	/// Replace the capsule with a character (default preview recipe).
	SetCharacter {
		species: CharacterSpecies,
	},
	#[command(subcommand)]
	Stats(Stats),
	/// Inspect or drive the Discovery sky clock.
	#[command(subcommand)]
	Sky(Sky),
	/// Internal 3D shading scale (`scale ×` window physical size).
	#[command(subcommand)]
	Render(Render),
	/// Debug mob placement (Tracy actor-density flights).
	#[command(subcommand)]
	Mobs(Mobs),
	/// Capture the primary window to a PNG via GPU readback.
	Screenshot {
		path: std::path::PathBuf,
	},
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Mobs {
	/// Place `count` hosts of one family on a square grid. Rosters are fixed by `--seed`.
	Spawn {
		kind: MobKindArg,
		count: u32,
		/// Grid center `X,Z`; defaults to the player.
		#[arg(long, value_parser = parse_xz, allow_hyphen_values = true)]
		at: Option<Vec2>,
		#[arg(long, default_value_t = 24.0)]
		spacing: f32,
		#[arg(long, default_value_t = 1)]
		seed: u32,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum MobKindArg {
	Herd,
	Pack,
	Raider,
	Guard,
	Pleb,
	Rambles,
	Brawler,
}

impl From<MobKindArg> for MobKind {
	fn from(kind: MobKindArg) -> Self {
		match kind {
			MobKindArg::Herd => Self::Herd,
			MobKindArg::Pack => Self::Pack,
			MobKindArg::Raider => Self::Raider,
			MobKindArg::Guard => Self::Guard,
			MobKindArg::Pleb => Self::Pleb,
			MobKindArg::Rambles => Self::Rambles,
			MobKindArg::Brawler => Self::Brawler,
		}
	}
}

fn parse_xz(raw: &str) -> Result<Vec2, String> {
	let (x, z) = raw.split_once(',').ok_or_else(|| format!("expected X,Z, got {raw:?}"))?;
	let x = x.trim().parse::<f32>().map_err(|error| format!("x: {error}"))?;
	let z = z.trim().parse::<f32>().map_err(|error| format!("z: {error}"))?;
	Ok(Vec2::new(x, z))
}

impl Mobs {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Mobs::Spawn { kind, count, at, spacing, seed } => {
				commands.spawn(RequestSpawnMobs {
					kind: kind.into(),
					count,
					at,
					spacing: spacing.max(1.0),
					seed,
				});
				*console = format!("mobs spawn {kind:?} x{count}: pending");
			}
		}
	}
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Mode {
	/// Free-look fly camera (WASD + mouse, Space/Shift vertical).
	Free,
	/// Capsule or character with third-person camera (WASD move, Space jump).
	Character,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Sky {
	/// Print phase, nearest preset, paused, and cycle length.
	Status,
	Dawn,
	Morning,
	Noon,
	Golden,
	Dusk,
	Night,
	/// Set phase in `0..1` (wraps). `0` midnight, `0.5` noon, `0.62` golden.
	At {
		phase: f32,
	},
	Pause,
	Play,
	/// Seconds per full day-night cycle.
	Rate {
		seconds: f32,
	},
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Render {
	/// Shade the 3D world at `factor ×` window physical size (`0.5`–`1.0`).
	Scale { factor: f32 },
	/// Keep the offscreen blit even at native resolution (`MAYBRAID_FORCE_OFFSCREEN`).
	Offscreen {
		#[arg(value_parser = parse_on_off, action = clap::ArgAction::Set)]
		enabled: bool,
	},
}

fn parse_on_off(raw: &str) -> Result<bool, String> {
	match raw.trim().to_ascii_lowercase().as_str() {
		"1" | "true" | "on" | "yes" => Ok(true),
		"0" | "false" | "off" | "no" => Ok(false),
		other => Err(format!("expected on/off, got {other:?}")),
	}
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Stats {
	/// Mesh triangle counts plus foliage / stick / structural LOD probe hosts.
	Mesh,
	/// Toggle the `[timing]` FPS log (and HUD when debug chrome is on).
	Fps,
	/// Toggle vsync (`AutoVsync` ↔ `Immediate`). Also `F8` / `MAYBRAID_VSYNC=off`.
	Vsync,
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn parse_startup_from_argv_tail(
		tail: Vec<std::ffi::OsString>,
	) -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_from_argv_tail(tail)
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			PlaygroundCommand::Help => *console = Self::long_help_string(),
			PlaygroundCommand::Script(s) => s.run(commands, console),
			PlaygroundCommand::Mode(mode) => mode.react(commands, console),
			PlaygroundCommand::SetCharacter { species } => {
				commands.spawn(RequestSetCharacter { species });
				*console = format!("set-character {}: pending", species.label());
			}
			PlaygroundCommand::Stats(stats) => stats.react(commands, console),
			PlaygroundCommand::Sky(sky) => sky.react(commands, console),
			PlaygroundCommand::Render(render) => render.react(commands, console),
			PlaygroundCommand::Mobs(mobs) => mobs.react(commands, console),
			PlaygroundCommand::Screenshot { path } => {
				commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
				*console = format!("screenshot {}: pending", path.display());
			}
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl Mode {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Mode::Free => {
				commands.spawn(RequestModeFree);
				*console = "mode free: pending".into();
			}
			Mode::Character => {
				commands.spawn(RequestModeCharacter);
				*console = "mode character: pending".into();
			}
		}
	}
}

impl Sky {
	fn react(self, commands: &mut Commands, console: &mut String) {
		let request = match self {
			Self::Status => SkyCommand::Status,
			Self::Dawn => SkyCommand::SetPhase(SKY_PHASE_DAWN),
			Self::Morning => SkyCommand::SetPhase(SKY_PHASE_MORNING),
			Self::Noon => SkyCommand::SetPhase(SKY_PHASE_NOON),
			Self::Golden => SkyCommand::SetPhase(SKY_PHASE_GOLDEN),
			Self::Dusk => SkyCommand::SetPhase(SKY_PHASE_DUSK),
			Self::Night => SkyCommand::SetPhase(SKY_PHASE_NIGHT),
			Self::At { phase } => SkyCommand::SetPhase(SkyClock::wrap_phase(phase)),
			Self::Pause => SkyCommand::SetPaused(true),
			Self::Play => SkyCommand::SetPaused(false),
			Self::Rate { seconds } => SkyCommand::SetPeriod(seconds.max(1.0)),
		};
		*console = match &request {
			SkyCommand::Status => "sky status: pending".into(),
			SkyCommand::SetPhase(phase) => format!("sky phase={phase:.3}"),
			SkyCommand::SetPaused(true) => "sky pause".into(),
			SkyCommand::SetPaused(false) => "sky play".into(),
			SkyCommand::SetPeriod(seconds) => format!("sky rate {seconds:.0}s"),
		};
		commands.spawn(request);
	}
}

impl Render {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Scale { factor } => {
				let factor = crate::render_scale::RenderScale::clamp(factor);
				commands.spawn(RequestRenderScale(factor));
				*console = format!("render scale {factor:.3}: pending");
			}
			Self::Offscreen { enabled } => {
				commands.spawn(RequestForceOffscreen(enabled));
				*console =
					format!("render offscreen {}: pending", if enabled { "on" } else { "off" });
			}
		}
	}
}

impl Stats {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Stats::Mesh => {
				commands.spawn(RequestMeshStats);
				*console = "stats mesh: pending".into();
			}
			Stats::Fps => {
				commands.spawn(RequestFpsToggle);
				*console = "stats fps: toggling".into();
			}
			Stats::Vsync => {
				commands.spawn(RequestVsyncToggle);
				*console = "stats vsync: toggling".into();
			}
		}
	}
}

pub(crate) fn apply_sky_commands(
	requests: Query<(Entity, &SkyCommand)>,
	mut clock: ResMut<SkyClock>,
	mut commands: Commands,
	mut console: Option<ResMut<CommandConsoleOutput>>,
) {
	for (entity, request) in &requests {
		match *request {
			SkyCommand::Status => {}
			SkyCommand::SetPhase(phase) => clock.phase = SkyClock::wrap_phase(phase),
			SkyCommand::SetPaused(paused) => clock.paused = paused,
			SkyCommand::SetPeriod(seconds) => clock.period_secs = seconds.max(1.0),
		}
		if let Some(console) = console.as_mut() {
			console.0 = clock.status_line();
		}
		commands.entity(entity).despawn();
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_stats_mesh() {
		let cmd = PlaygroundCommand::parse_line("stats mesh").unwrap();
		assert!(matches!(cmd, PlaygroundCommand::Stats(Stats::Mesh)));
	}

	#[test]
	fn parse_stats_vsync() {
		let cmd = PlaygroundCommand::parse_line("stats vsync").unwrap();
		assert!(matches!(cmd, PlaygroundCommand::Stats(Stats::Vsync)));
	}

	#[test]
	fn parse_render_scale() -> anyhow::Result<()> {
		let cmd = PlaygroundCommand::parse_line("render scale 0.75")
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		match cmd {
			PlaygroundCommand::Render(Render::Scale { factor }) => {
				anyhow::ensure!((factor - 0.75).abs() < 1e-4, "factor {factor}");
			}
			_ => anyhow::bail!("expected render scale 0.75"),
		}
		Ok(())
	}

	#[test]
	fn parse_render_offscreen() -> anyhow::Result<()> {
		let on = PlaygroundCommand::parse_line("render offscreen on")
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		assert!(matches!(on, PlaygroundCommand::Render(Render::Offscreen { enabled: true })));
		let off = PlaygroundCommand::parse_line("render offscreen off")
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		assert!(matches!(off, PlaygroundCommand::Render(Render::Offscreen { enabled: false })));
		Ok(())
	}

	#[test]
	fn parse_sky_presets() {
		let noon = PlaygroundCommand::parse_line("sky noon").unwrap();
		assert!(matches!(noon, PlaygroundCommand::Sky(Sky::Noon)));
		let at = PlaygroundCommand::parse_line("sky at 0.35").unwrap();
		assert!(
			matches!(at, PlaygroundCommand::Sky(Sky::At { phase }) if (phase - 0.35).abs() < 1e-4)
		);
		let rate = PlaygroundCommand::parse_line("sky rate 1200").unwrap();
		assert!(
			matches!(rate, PlaygroundCommand::Sky(Sky::Rate { seconds }) if (seconds - 1200.0).abs() < 1e-3)
		);
	}

	#[test]
	fn parse_mobs_spawn() {
		let cmd =
			PlaygroundCommand::parse_line("mobs spawn pleb 6 --at 40,-12.5 --seed 3").unwrap();
		let PlaygroundCommand::Mobs(Mobs::Spawn { kind, count, at, spacing, seed }) = cmd else {
			panic!("expected mobs spawn");
		};
		assert_eq!(kind, MobKindArg::Pleb);
		assert_eq!(count, 6);
		assert_eq!(at, Some(Vec2::new(40.0, -12.5)));
		assert_eq!(spacing, 24.0);
		assert_eq!(seed, 3);
		let west = PlaygroundCommand::parse_line("mobs spawn herd 1 --at -40,8").unwrap();
		assert!(matches!(
			west,
			PlaygroundCommand::Mobs(Mobs::Spawn { at: Some(at), .. }) if at == Vec2::new(-40.0, 8.0)
		));
		assert!(PlaygroundCommand::parse_line("mobs spawn pleb 2 --at 4").is_err());
	}

	#[test]
	fn parse_mode_and_set_character() {
		let cmd = PlaygroundCommand::parse_line("mode character").unwrap();
		assert!(matches!(cmd, PlaygroundCommand::Mode(Mode::Character)));
		let set = PlaygroundCommand::parse_line("set-character braidman").unwrap();
		assert!(matches!(
			set,
			PlaygroundCommand::SetCharacter { species: CharacterSpecies::Braidman }
		));
	}

	#[test]
	fn parse_screenshot_path() {
		let cmd = PlaygroundCommand::parse_line("screenshot /tmp/canopy-noon.png").unwrap();
		let PlaygroundCommand::Screenshot { path } = cmd else {
			panic!("expected screenshot");
		};
		assert_eq!(path, std::path::PathBuf::from("/tmp/canopy-noon.png"));
	}
}
