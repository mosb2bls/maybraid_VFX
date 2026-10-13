//! `/epiphant` commands for the elephant-like quadruped concept species.

use bevy::prelude::*;
use characters::{
	species::{
		common::EyeMesh,
		epiphant::{
			assets::{EpiphantBodyMesh, EpiphantNoseMesh},
			sliders::EpiphantSliders,
			EpiphantConfig,
		},
	},
	BuildPreset, GenderPreset,
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Epiphant {
	/// Spawn an Epiphant quadruped through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = GenderPreset::Neutral)]
	pub gender: GenderPreset,

	#[arg(long, value_enum, default_value_t = BuildPreset::Average)]
	pub build: BuildPreset,

	#[arg(long, value_enum, default_value_t = EpiphantBodyMesh::Epiphant)]
	pub body: EpiphantBodyMesh,

	#[arg(long, value_enum, default_value_t = EpiphantNoseMesh::Trunkish)]
	pub nose: EpiphantNoseMesh,

	#[arg(long, value_enum, default_value_t = EyeMesh::Standard)]
	pub eye: EyeMesh,

	#[arg(long, default_value_t = 1.0)]
	pub shoulder_width: f32,

	#[arg(long, default_value_t = 1.0)]
	pub hip_width: f32,

	#[arg(long, default_value_t = 1.0)]
	pub chest_thickness: f32,

	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,
}

impl Epiphant {
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
		let sliders = EpiphantSliders::default()
			.with_shoulder_width(self.shoulder_width)
			.with_hip_width(self.hip_width)
			.with_chest_thickness(self.chest_thickness);
		ConceptPreviewConfig::epiphant_with_animation(
			EpiphantConfig {
				gender: self.gender,
				build: self.build,
				body: self.body,
				head: Default::default(),
				ear: Default::default(),
				nose: self.nose,
				eye: self.eye,
				colors: Default::default(),
				sliders,
			},
			self.animation,
		)
	}
}
