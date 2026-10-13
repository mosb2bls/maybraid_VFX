//! `/thumplus` commands for the Thumplus concept species.

use bevy::prelude::*;
use characters::species::thumplus::{ThumplusBodyColor, ThumplusColors, ThumplusConfig};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Thumplus {
	/// Spawn a Thumplus through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	#[arg(long, value_enum, default_value_t = ThumplusBodyColor::Ocean)]
	pub body: ThumplusBodyColor,
}

impl Thumplus {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Self::Preview(args) => {
				let config = args.into_preview_config();
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<ConceptPreviewConfig>() = config;
				});
			}
		}
	}
}

impl PreviewArgs {
	fn into_preview_config(self) -> ConceptPreviewConfig {
		ConceptPreviewConfig::thumplus_with_animation(
			ThumplusConfig { colors: ThumplusColors { body: self.body } },
			self.animation,
		)
	}
}
