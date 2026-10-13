//! In-game clap commands for the bump-out playground.

use bevy::prelude::*;
use clap::{Args, Parser, Subcommand, ValueEnum};
use game_commands::command::{CommandScript, GameCommand};

use crate::PresenterLayer;

pub const PLAYGROUND_CLI_NAME: &str = "chico-bumpout";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "chico-bumpout",
	version,
	about = "Chico bump-out playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	#[command(subcommand)]
	Neighborhood(NeighborhoodCommand),
	#[command(subcommand)]
	Visibility(VisibilityCommand),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum NeighborhoodCommand {
	/// Choose which bump-out layer subsequent edits affect.
	Layer {
		#[arg(value_enum)]
		layer: BumpOutLayer,
	},
	/// Select a world sample in the center tile's 3×3 neighborhood.
	Select {
		#[arg(
			long,
			allow_hyphen_values = true,
			value_parser = clap::value_parser!(i32).range(-1..=1)
		)]
		x: i32,
		#[arg(
			long,
			allow_hyphen_values = true,
			value_parser = clap::value_parser!(i32).range(-1..=1)
		)]
		z: i32,
	},
	/// Set one or more values on the selected world sample.
	Set(NeighborhoodValues),
	/// Add to one or more values on the selected world sample.
	Adjust(NeighborhoodValues),
}

#[derive(Clone, Args, Debug, Default)]
#[command(rename_all = "kebab-case")]
pub struct NeighborhoodValues {
	#[arg(long)]
	pub density: Option<f32>,
	/// Characteristic fragment-bite diameter in world units.
	#[arg(long)]
	pub bite_size: Option<f32>,
	/// Symmetric bite-size variation in binary scale octaves.
	#[arg(long)]
	pub bite_size_deviation: Option<f32>,
	#[arg(long)]
	pub average_height: Option<f32>,
	/// Symmetric vertical displacement in world units.
	#[arg(long)]
	pub height_deviation: Option<f32>,
}

impl NeighborhoodValues {
	pub fn is_empty(&self) -> bool {
		self.density.is_none()
			&& self.bite_size.is_none()
			&& self.bite_size_deviation.is_none()
			&& self.average_height.is_none()
			&& self.height_deviation.is_none()
	}
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum BumpOutLayer {
	GroundCover,
	CanopyProxy,
}

impl From<BumpOutLayer> for PresenterLayer {
	fn from(layer: BumpOutLayer) -> Self {
		match layer {
			BumpOutLayer::GroundCover => Self::GroundCover,
			BumpOutLayer::CanopyProxy => Self::CanopyProxy,
		}
	}
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum VisibilityCommand {
	Show {
		#[arg(value_enum)]
		layer: VisibleLayer,
	},
	Hide {
		#[arg(value_enum)]
		layer: VisibleLayer,
	},
	Toggle {
		#[arg(value_enum)]
		layer: VisibleLayer,
	},
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum VisibleLayer {
	Terrain,
	GroundCover,
	CanopyProxy,
}

impl From<VisibleLayer> for PresenterLayer {
	fn from(layer: VisibleLayer) -> Self {
		match layer {
			VisibleLayer::Terrain => Self::Terrain,
			VisibleLayer::GroundCover => Self::GroundCover,
			VisibleLayer::CanopyProxy => Self::CanopyProxy,
		}
	}
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
			Self::Neighborhood(command) => command.react(commands, console),
			Self::Visibility(command) => command.react(commands, console),
		}
	}
}

impl NeighborhoodCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Layer { layer } => {
				commands.queue(move |world: &mut World| {
					world.resource_mut::<crate::NeighborhoodControls>().layer = layer.into();
				});
				*console = format!("editing {} bump-outs", PresenterLayer::from(layer).label());
			}
			Self::Select { x, z } => {
				commands.queue(move |world: &mut World| {
					let mut controls = world.resource_mut::<crate::NeighborhoodControls>();
					controls.column = (x + 1) as usize;
					controls.row = (z + 1) as usize;
				});
				*console = format!("selected center-neighborhood sample ({x}, {z})");
			}
			Self::Set(values) => {
				if values.is_empty() {
					*console = "neighborhood set requires at least one value".into();
					return;
				}
				commands.queue(move |world: &mut World| {
					crate::apply_neighborhood_edit(world, &values, false);
				});
				*console = "set selected neighborhood sample".into();
			}
			Self::Adjust(values) => {
				if values.is_empty() {
					*console = "neighborhood adjust requires at least one value".into();
					return;
				}
				commands.queue(move |world: &mut World| {
					crate::apply_neighborhood_edit(world, &values, true);
				});
				*console = "adjusted selected neighborhood sample".into();
			}
		}
	}
}

impl VisibilityCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		let (layer, change) = match self {
			Self::Show { layer } => (layer, Some(true)),
			Self::Hide { layer } => (layer, Some(false)),
			Self::Toggle { layer } => (layer, None),
		};
		commands.queue(move |world: &mut World| {
			crate::change_layer_visibility(world, layer.into(), change);
		});
		*console = format!("updated {} visibility", PresenterLayer::from(layer).label());
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
	fn parses_complete_neighborhood_set() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"neighborhood set --density 0.6 --bite-size 14 \
			 --bite-size-deviation 0.75 --average-height 32 --height-deviation 6",
		)?;
		let PlaygroundCommand::Neighborhood(NeighborhoodCommand::Set(values)) = command else {
			return Err("expected neighborhood set".into());
		};
		assert_eq!(values.density, Some(0.6));
		assert_eq!(values.bite_size, Some(14.0));
		assert_eq!(values.bite_size_deviation, Some(0.75));
		assert_eq!(values.average_height, Some(32.0));
		assert_eq!(values.height_deviation, Some(6.0));
		Ok(())
	}

	#[test]
	fn constrains_selected_sample_to_center_neighborhood() {
		assert!(<PlaygroundCommand as GameCommand>::parse_line("neighborhood select --x -1 --z 1")
			.is_ok());
		assert!(<PlaygroundCommand as GameCommand>::parse_line("neighborhood select --x 2 --z 0")
			.is_err());
	}
}
