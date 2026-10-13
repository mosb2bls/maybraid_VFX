//! `/mygr` commands for the Mygr concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	mygr::{MygrConfig, MygrEyeColor, MygrSkinColor},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Mygr {
	/// Spawn a Mygr through the resolved concepts pipeline.
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

	#[arg(long, value_enum, default_value_t = MygrSkinColor::Ginger)]
	pub skin: MygrSkinColor,

	#[arg(long, value_enum, default_value_t = MygrEyeColor::Green)]
	pub eyes: MygrEyeColor,
}

impl Mygr {
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
		let mut colors = characters::species::mygr::MygrColors::default();
		colors.skin = self.skin;
		colors.eyes = self.eyes;
		ConceptPreviewConfig::mygr_with_animation(
			MygrConfig { eye: self.eye, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
