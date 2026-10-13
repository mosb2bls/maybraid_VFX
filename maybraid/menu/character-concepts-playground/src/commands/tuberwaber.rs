//! `/tuberwaber` commands for the biped concept species.

use bevy::prelude::*;
use character_items::{ClothingMaterial, ClothingMesh};
use characters::{
	species::{
		common::{EyeMesh, HairMesh, MouthMesh, NoseMesh},
		tuberwaber::{
			sliders::TuberwaberSliders, TuberwaberBodyMesh, TuberwaberConfig, TuberwaberHeadMesh,
		},
	},
	BuildPreset, GenderPreset,
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Tuberwaber {
	/// Spawn a Tuberwaber biped through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = GenderPreset::Neutral)]
	pub gender: GenderPreset,

	#[arg(long, value_enum, default_value_t = BuildPreset::Average)]
	pub build: BuildPreset,

	#[arg(long, value_enum, default_value_t = TuberwaberBodyMesh::Tuberwaber)]
	pub body: TuberwaberBodyMesh,

	#[arg(long, value_enum, default_value_t = TuberwaberHeadMesh::Tuberwaber)]
	pub head: TuberwaberHeadMesh,

	#[arg(long, value_enum, default_value_t = EyeMesh::Standard)]
	pub eye: EyeMesh,

	#[arg(long, value_enum, default_value_t = NoseMesh::Loaf)]
	pub nose: NoseMesh,

	#[arg(long, value_enum, default_value_t = MouthMesh::Standard)]
	pub mouth: MouthMesh,

	#[arg(long, value_enum, default_value_t = HairMesh::None)]
	pub hair: HairMesh,

	/// Clothing layers to remap to the body rig. Repeat the flag for multiple layers.
	#[arg(long, value_enum)]
	pub clothing: Vec<ClothingMesh>,

	/// Default surface recipe for worn layers without a per-item override.
	#[arg(long, value_enum, default_value_t = ClothingMaterial::Cloth)]
	pub clothing_material: ClothingMaterial,

	/// Procedural body animation used to inspect sockets and skinning under motion.
	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	/// Shoulder width multiplier on top of the Tuberwaber species baseline.
	#[arg(long, default_value_t = 1.0)]
	pub shoulder_width: f32,

	/// Hip width multiplier on top of the Tuberwaber species baseline.
	#[arg(long, default_value_t = 1.0)]
	pub hip_width: f32,

	/// Chest thickness multiplier on top of the Tuberwaber species baseline.
	#[arg(long, default_value_t = 1.0)]
	pub chest_thickness: f32,
}

impl Tuberwaber {
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
		let sliders = TuberwaberSliders::default()
			.with_shoulder_width(self.shoulder_width)
			.with_hip_width(self.hip_width)
			.with_chest_thickness(self.chest_thickness);
		ConceptPreviewConfig::tuberwaber_with_animation(
			TuberwaberConfig {
				gender: self.gender,
				build: self.build,
				body: self.body,
				head: self.head,
				eye: self.eye,
				nose: self.nose,
				mouth: self.mouth,
				hair: self.hair,
				clothing: self.clothing,
				colors: {
					let mut colors = characters::species::tuberwaber::TuberwaberColors::default();
					colors.clothing_material = self.clothing_material;
					colors
				},
				sliders,
			},
			self.animation,
		)
	}
}
