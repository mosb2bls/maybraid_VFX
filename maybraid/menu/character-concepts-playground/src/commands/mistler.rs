//! `/mistler` commands for the Mistler concept species.

use bevy::prelude::*;
use characters::species::mistler::{MistlerBodyColor, MistlerColors, MistlerConfig};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Mistler {
	/// Spawn a Mistler through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	#[arg(long, value_enum, default_value_t = MistlerBodyColor::Coral)]
	pub body: MistlerBodyColor,
}

impl Mistler {
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
		ConceptPreviewConfig::mistler_with_animation(
			MistlerConfig { colors: MistlerColors { body: self.body } },
			self.animation,
		)
	}
}
