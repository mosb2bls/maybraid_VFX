//! `/wumbus` commands for the Wumbus concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	wumbus::{WumbusConfig, WumbusEarColor, WumbusEyeColor, WumbusHornMesh, WumbusSkinColor},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Wumbus {
	/// Spawn a Wumbus through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = WumbusHornMesh::None)]
	pub horns: WumbusHornMesh,

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

	#[arg(long, value_enum, default_value_t = WumbusSkinColor::Chocolate)]
	pub skin: WumbusSkinColor,

	#[arg(long, value_enum, default_value_t = WumbusEyeColor::PaleBlue)]
	pub eyes: WumbusEyeColor,

	#[arg(long, value_enum, default_value_t = WumbusEarColor::Cream)]
	pub ears: WumbusEarColor,
}

impl Wumbus {
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
		let mut colors = characters::species::wumbus::WumbusColors::default();
		colors.skin = self.skin;
		colors.eyes = self.eyes;
		colors.ears = self.ears;
		ConceptPreviewConfig::wumbus_with_animation(
			WumbusConfig {
				horns: self.horns,
				eye: self.eye,
				hair: self.hair,
				clothing: self.clothing,
				colors,
			},
			self.animation,
		)
	}
}
