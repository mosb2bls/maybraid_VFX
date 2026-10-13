//! `/brokker` commands for the Brokker concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	brokker::{BrokkerConfig, BrokkerEyeColor, BrokkerPlumageColor, BrokkerSnoutColor},
	common::{EyeMesh, HairMesh},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Brokker {
	/// Spawn a Brokker through the resolved concepts pipeline.
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

	#[arg(long, value_enum, default_value_t = BrokkerPlumageColor::Olive)]
	pub plumage: BrokkerPlumageColor,

	#[arg(long, value_enum, default_value_t = BrokkerEyeColor::Amber)]
	pub eyes: BrokkerEyeColor,

	#[arg(long, value_enum, default_value_t = BrokkerSnoutColor::Horn)]
	pub snout_color: BrokkerSnoutColor,
}

impl Brokker {
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
		let mut colors = characters::species::brokker::BrokkerColors::default();
		colors.plumage = self.plumage;
		colors.eyes = self.eyes;
		colors.snout = self.snout_color;
		ConceptPreviewConfig::brokker_with_animation(
			BrokkerConfig { eye: self.eye, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
