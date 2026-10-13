//! `/grener` commands for the Grener concept species.

use bevy::prelude::*;
use characters::species::grener::{GrenerBodyColor, GrenerColors, GrenerConfig};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Grener {
	/// Spawn a Grener through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	#[arg(long, value_enum, default_value_t = GrenerBodyColor::Slate)]
	pub body: GrenerBodyColor,
}

impl Grener {
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
		ConceptPreviewConfig::grener_with_animation(
			GrenerConfig { colors: GrenerColors { body: self.body } },
			self.animation,
		)
	}
}
