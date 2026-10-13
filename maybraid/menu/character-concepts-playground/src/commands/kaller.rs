//! `/kaller` commands for the Kaller concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::{EyeMesh, HairMesh},
	kaller::{
		KallerConfig, KallerCrownColor, KallerEyeColor, KallerPlumageColor, KallerSnoutColor,
	},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Kaller {
	/// Spawn a Kaller through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = EyeMesh::Falcon)]
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

	#[arg(long, value_enum, default_value_t = KallerPlumageColor::Olive)]
	pub plumage: KallerPlumageColor,

	#[arg(long, value_enum, default_value_t = KallerEyeColor::Amber)]
	pub eyes: KallerEyeColor,

	#[arg(long, value_enum, default_value_t = KallerSnoutColor::Horn)]
	pub snout_color: KallerSnoutColor,

	#[arg(long, value_enum, default_value_t = KallerCrownColor::Charcoal)]
	pub crown_color: KallerCrownColor,
}

impl Kaller {
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
		let mut colors = characters::species::kaller::KallerColors::default();
		colors.plumage = self.plumage;
		colors.eyes = self.eyes;
		colors.snout = self.snout_color;
		colors.crown = self.crown_color;
		ConceptPreviewConfig::kaller_with_animation(
			KallerConfig { eye: self.eye, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
