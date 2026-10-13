//! `/dui` commands for the Dui concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::HairMesh,
	dui::{DuiConfig, DuiMouthColor, DuiNoseMesh, DuiSkinColor},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Dui {
	/// Spawn a Dui through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = DuiNoseMesh::None)]
	pub nose: DuiNoseMesh,

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

	#[arg(long, value_enum, default_value_t = DuiSkinColor::Purple)]
	pub skin: DuiSkinColor,

	#[arg(long, value_enum, default_value_t = DuiMouthColor::Red)]
	pub mouth_color: DuiMouthColor,
}

impl Dui {
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
		let mut colors = characters::species::dui::DuiColors::default();
		colors.skin = self.skin;
		colors.mouth = self.mouth_color;
		ConceptPreviewConfig::dui_with_animation(
			DuiConfig { nose: self.nose, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
