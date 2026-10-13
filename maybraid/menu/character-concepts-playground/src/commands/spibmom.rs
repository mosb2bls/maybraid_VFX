//! `/spibmom` commands for the Spibmom concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	spibmom::{
		SpibmomConfig, SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomMouthColor,
		SpibmomSkinColor, SpibmomSpineColor,
	},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Spibmom {
	/// Spawn a Spibmom through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = EyeMesh::Standard)]
	pub eye: EyeMesh,

	#[arg(long, value_enum, default_value_t = HairMesh::None)]
	pub hair: HairMesh,

	/// Clothing layers to remap to the body rig. Repeat the flag for multiple layers.
	#[arg(long, value_enum)]
	pub clothing: Vec<ClothingMesh>,

	/// Default surface recipe for worn layers without a per-item override.
	#[arg(long, value_enum, default_value_t = ClothingMaterial::Cloth)]
	pub clothing_material: ClothingMaterial,

	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	#[arg(long, value_enum, default_value_t = SpibmomSkinColor::PowderBlue)]
	pub skin: SpibmomSkinColor,

	#[arg(long, value_enum, default_value_t = SpibmomEyeColor::Pearl)]
	pub eyes: SpibmomEyeColor,

	#[arg(long, value_enum, default_value_t = SpibmomEarColor::Umber)]
	pub ears: SpibmomEarColor,

	#[arg(long, value_enum, default_value_t = SpibmomMouthColor::Espresso)]
	pub nose_color: SpibmomMouthColor,

	#[arg(long, value_enum, default_value_t = SpibmomCrownColor::Charcoal)]
	pub crown: SpibmomCrownColor,

	#[arg(long, value_enum, default_value_t = SpibmomSpineColor::Charcoal)]
	pub spine: SpibmomSpineColor,
}

impl Spibmom {
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
		let mut colors = characters::species::spibmom::SpibmomColors::default();
		colors.skin = self.skin;
		colors.eyes = self.eyes;
		colors.ears = self.ears;
		colors.mouth = self.nose_color;
		colors.crown = self.crown;
		colors.spine = self.spine;
		ConceptPreviewConfig::spibmom_with_animation(
			SpibmomConfig { eye: self.eye, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
