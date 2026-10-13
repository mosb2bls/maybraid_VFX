//! In-game clap command hierarchy for the Durham terrain models playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "durham-terrain";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "durham-terrain",
	version,
	about = "Durham terrain models playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Configure the fixed terrain cell request region and regenerate.
	#[command(subcommand)]
	Cells(Cells),
	/// Switch between free-look fly camera and third-person character control.
	#[command(subcommand)]
	Mode(Mode),
	/// Replace the capsule with a character (default preview recipe).
	SetCharacter {
		/// Species id (`braidman`, `mygr`, `hars`, …).
		species: crate::character::CharacterSpecies,
	},
	/// LOD / mesh CPU proxies (triangle counts, etc.).
	#[command(subcommand)]
	Stats(Stats),
	/// Set the world generation seed and regenerate terrain + water.
	Seed {
		/// World seed (jersey families offset from this; Watershed uses it directly).
		value: u32,
	},
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Cells {
	/// Print the current cell size, origin, and extents.
	Show,
	/// Set cell layout fields (omit a flag to keep the current value) and regenerate.
	Set {
		/// Edge length of each origin cell in world units.
		#[arg(long)]
		size: Option<f32>,
		/// Cell-grid origin X (min corner).
		#[arg(long, allow_hyphen_values = true)]
		origin_x: Option<i32>,
		/// Cell-grid origin Z (min corner).
		#[arg(long, allow_hyphen_values = true)]
		origin_z: Option<i32>,
		/// Number of cells along +X.
		#[arg(long)]
		extent_x: Option<u32>,
		/// Number of cells along +Z.
		#[arg(long)]
		extent_z: Option<u32>,
	},
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
pub enum Stats {
	/// Sum vertex / index / triangle counts from spawned `Mesh3d` assets (excludes player).
	Mesh,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestCellShow;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeFree;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeCharacter;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestMeshStats;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestSeed(pub u32);

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			PlaygroundCommand::Help => *console = Self::long_help_string(),
			PlaygroundCommand::Script(s) => s.run(commands, console),
			PlaygroundCommand::Cells(cells) => cells.react(commands, console),
			PlaygroundCommand::Mode(mode) => mode.react(commands, console),
			PlaygroundCommand::SetCharacter { species } => {
				commands.spawn(crate::character::RequestSetCharacter { species });
				*console = format!("set-character {}: pending", species.label());
			}
			PlaygroundCommand::Stats(stats) => stats.react(commands, console),
			PlaygroundCommand::Seed { value } => {
				commands.spawn(RequestSeed(value));
				*console = format!("seed {value}: regenerating");
			}
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl Cells {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Cells::Show => {
				commands.spawn(RequestCellShow);
				*console = "cells show: pending".into();
			}
			Cells::Set { size, origin_x, origin_z, extent_x, extent_z } => {
				commands.spawn(PendingCellLayoutPatch {
					size,
					origin_x,
					origin_z,
					extent_x,
					extent_z,
				});
				*console = "cells set: regenerating".into();
			}
		}
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

impl Stats {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Stats::Mesh => {
				commands.spawn(RequestMeshStats);
				*console = "stats mesh: pending".into();
			}
		}
	}
}

/// Partial update applied against the current [`TerrainCellLayout`].
#[derive(Component, Debug, Clone)]
pub struct PendingCellLayoutPatch {
	pub size: Option<f32>,
	pub origin_x: Option<i32>,
	pub origin_z: Option<i32>,
	pub extent_x: Option<u32>,
	pub extent_z: Option<u32>,
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
	fn parse_seed() {
		let cmd = PlaygroundCommand::parse_line("seed 7").expect("parse");
		assert!(matches!(cmd, PlaygroundCommand::Seed { value: 7 }));
	}
}
