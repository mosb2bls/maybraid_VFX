//! `/lero` commands for the Lero concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::species::{
	common::HairMesh,
	lero::{
		LeroConfig, LeroEyeColor, LeroMouthColor, LeroMouthMesh, LeroSkinColor, LeroSpineColor,
		LeroTailColor,
	},
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Lero {
	/// Spawn a Lero through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = LeroMouthMesh::Lerodon)]
	pub mouth: LeroMouthMesh,

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

	#[arg(long, value_enum, default_value_t = LeroSkinColor::FadedGreen)]
	pub skin: LeroSkinColor,

	#[arg(long, value_enum, default_value_t = LeroEyeColor::Gold)]
	pub eyes: LeroEyeColor,

	#[arg(long, value_enum, default_value_t = LeroMouthColor::SoftBlush)]
	pub snout_color: LeroMouthColor,

	#[arg(long, value_enum, default_value_t = LeroTailColor::Pearl)]
	pub tail: LeroTailColor,

	#[arg(long, value_enum, default_value_t = LeroSpineColor::Pearl)]
	pub spine: LeroSpineColor,
}

impl Lero {
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
		let mut colors = characters::species::lero::LeroColors::default();
		colors.skin = self.skin;
		colors.eyes = self.eyes;
		colors.mouth = self.snout_color;
		colors.tail = self.tail;
		colors.spine = self.spine;
		ConceptPreviewConfig::lero_with_animation(
			LeroConfig { mouth: self.mouth, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
