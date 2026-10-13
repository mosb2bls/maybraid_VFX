//! `/tipple` commands for the Tipple concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	tipple::{TippleBeakColor, TippleBeakMesh, TippleConfig, TippleEyeColor, TipplePlumageColor},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Tipple {
	/// Spawn a Tipple through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = TippleBeakMesh::Hook)]
	pub beak: TippleBeakMesh,

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

	#[arg(long, value_enum, default_value_t = TipplePlumageColor::Yellow)]
	pub plumage: TipplePlumageColor,

	#[arg(long, value_enum, default_value_t = TippleEyeColor::Sky)]
	pub eyes: TippleEyeColor,

	#[arg(long, value_enum, default_value_t = TippleBeakColor::Orange)]
	pub beak_color: TippleBeakColor,
}

impl Tipple {
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
		let mut colors = characters::species::tipple::TippleColors::default();
		colors.plumage = self.plumage;
		colors.eyes = self.eyes;
		colors.beak = self.beak_color;
		ConceptPreviewConfig::tipple_with_animation(
			TippleConfig {
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
