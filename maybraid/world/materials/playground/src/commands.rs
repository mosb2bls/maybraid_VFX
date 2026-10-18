//! In-game clap commands for the world-materials playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use firearms::{laser_hex_ref, laser_pulse_ref, laser_tail_ref, muzzle_flame_ref};
use game_commands::command::{CommandScript, GameCommand};
use material_ref::MaterialRef;

use crate::preview::PreviewConfig;
use crate::shape::PreviewShape;
use crate::shoot::ShootConfig;

pub const PLAYGROUND_CLI_NAME: &str = "materials";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "materials",
	version,
	about = "World materials playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Put a named recipe on the current preview hull.
	Show {
		/// `hex`, `pulse`, `tail`, `muzzle-flame`, `standard`, or any world recipe name.
		recipe: String,
	},
	/// Switch the preview / shot hull. `sphere` and `capsule` are the solid baselines.
	Mesh {
		/// `sphere`, `capsule`, `core`, `shell`, `ring`, `cards`, or `blast`.
		shape: PreviewShape,
	},
	/// Fire repeating copies of the current hull. `shoot stop` cancels.
	Shoot {
		#[command(subcommand)]
		action: Option<ShootAction>,
	},
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ShootAction {
	/// Stop the repeating launcher.
	Stop,
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
			Self::Show { recipe } => {
				let material = material_for_show(&recipe);
				*console = format!("show {}", preview_label(&material));
				commands.queue(move |world: &mut World| {
					world.resource_mut::<PreviewConfig>().material = material;
				});
			}
			Self::Mesh { shape } => {
				*console = format!("mesh {}", shape.label());
				commands.queue(move |world: &mut World| {
					world.resource_mut::<PreviewConfig>().shape = shape;
				});
			}
			Self::Shoot { action: Some(ShootAction::Stop) } => {
				*console = "shoot stop".into();
				commands.queue(|world: &mut World| {
					world.resource_mut::<ShootConfig>().enabled = false;
				});
			}
			Self::Shoot { action: None } => {
				*console = "shoot".into();
				commands.queue(|world: &mut World| {
					let preview = world.resource::<PreviewConfig>().clone();
					let mut shoot = world.resource_mut::<ShootConfig>();
					shoot.enabled = true;
					shoot.material = preview.material;
					shoot.shape = preview.shape;
					shoot.accumulator = shoot.interval;
				});
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

pub(crate) fn material_for_show(name: &str) -> MaterialRef {
	match name {
		"hex" | "laser_hex" | "laser-hex" => laser_hex_ref(),
		"pulse" | "laser_pulse" | "laser-pulse" => laser_pulse_ref(),
		"tail" | "laser_tail" | "laser-tail" => laser_tail_ref(),
		"muzzle-flame" | "muzzle_flame" => muzzle_flame_ref(),
		"standard" | "default" => MaterialRef::default(),
		other => MaterialRef::named(other),
	}
}

pub(crate) fn preview_label(material: &MaterialRef) -> String {
	match &material.name {
		material_ref::MaterialId::Default => "standard".into(),
		material_ref::MaterialId::Name(name) => name.clone(),
	}
}
