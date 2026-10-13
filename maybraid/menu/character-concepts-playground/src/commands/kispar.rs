//! `/kispar` commands for the Kispar concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	kispar::{KisparBeakColor, KisparBeakMesh, KisparConfig, KisparEyeColor, KisparPlumageColor},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Kispar {
	/// Spawn a Kispar through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = KisparBeakMesh::Hook)]
	pub beak: KisparBeakMesh,

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

	#[arg(long, value_enum, default_value_t = KisparPlumageColor::Ash)]
	pub plumage: KisparPlumageColor,

	#[arg(long, value_enum, default_value_t = KisparEyeColor::SoftAmber)]
	pub eyes: KisparEyeColor,

	#[arg(long, value_enum, default_value_t = KisparBeakColor::Horn)]
	pub beak_color: KisparBeakColor,
}

impl Kispar {
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
		let mut colors = characters::species::kispar::KisparColors::default();
		colors.plumage = self.plumage;
		colors.eyes = self.eyes;
		colors.beak = self.beak_color;
		ConceptPreviewConfig::kispar_with_animation(
			KisparConfig {
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
