//! `/tapp` commands for the Tapp concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	tapp::{TappBeakColor, TappBeakMesh, TappConfig, TappEyeColor, TappPlumageColor},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Tapp {
	/// Spawn a Tapp through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = TappBeakMesh::Sharp)]
	pub beak: TappBeakMesh,

	#[arg(long, value_enum, default_value_t = EyeMesh::Falcon)]
	pub eye: EyeMesh,

	#[arg(long, value_enum, default_value_t = HairMesh::FeatherHawk)]
	pub hair: HairMesh,

	/// Clothing layers to remap to the body rig. Repeat the flag for multiple layers.
	#[arg(long, value_enum)]
	pub clothing: Vec<ClothingMesh>,

	/// Default surface recipe for worn layers without a per-item override.
	#[arg(long, value_enum, default_value_t = ClothingMaterial::Cloth)]
	pub clothing_material: ClothingMaterial,

	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	#[arg(long, value_enum, default_value_t = TappPlumageColor::Mist)]
	pub plumage: TappPlumageColor,

	#[arg(long, value_enum, default_value_t = TappEyeColor::SoftBlue)]
	pub eyes: TappEyeColor,

	#[arg(long, value_enum, default_value_t = TappBeakColor::Slate)]
	pub beak_color: TappBeakColor,
}

impl Tapp {
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
		let mut colors = characters::species::tapp::TappColors::default();
		colors.plumage = self.plumage;
		colors.eyes = self.eyes;
		colors.beak = self.beak_color;
		ConceptPreviewConfig::tapp_with_animation(
			TappConfig {
				beak: self.beak,
				eye: self.eye,
				hair: self.hair,
				clothing: self.clothing,
				colors,
			},
			self.animation,
		)
	}
}
