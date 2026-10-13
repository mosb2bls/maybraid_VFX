//! `/brodler` commands for the Brodler concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	brodler::{
		assets::HornMesh,
		{BrodlerConfig, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor},
	},
	common::{EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Brodler {
	/// Spawn a Brodler through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = BrodlerHeadMesh::Gaunt)]
	pub head: BrodlerHeadMesh,

	#[arg(long, value_enum, default_value_t = HornMesh::HarrowedCrown)]
	pub horns: HornMesh,

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

	#[arg(long, value_enum, default_value_t = BrodlerSkinColor::Crimson)]
	pub skin: BrodlerSkinColor,

	#[arg(long, value_enum, default_value_t = BrodlerEyeColor::LightBlue)]
	pub eyes: BrodlerEyeColor,

	#[arg(long, value_enum, default_value_t = BrodlerHornColor::LightBrown)]
	pub horn_color: BrodlerHornColor,
}

impl Brodler {
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
		let mut colors = characters::species::brodler::BrodlerColors::default();
		colors.skin = self.skin;
		colors.eyes = self.eyes;
		colors.horns = self.horn_color;
		ConceptPreviewConfig::brodler_with_animation(
			BrodlerConfig {
				head: self.head,
				horns: self.horns,
				eye: EyeMesh::Standard,
				nose: NoseMesh::Standard,
				mouth: MouthMesh::Standard,
				ear: EarMesh::Flank,
				hair: self.hair,
				clothing: self.clothing,
				colors,
			},
			self.animation,
		)
	}
}
