//! `/claber` commands for the oversized low-slung quadruped concept species.

use bevy::prelude::*;
use characters::{
	species::{
		claber::{sliders::ClaberSliders, ClaberConfig, ClaberHornMesh},
		common::EyeMesh,
	},
	BuildPreset, GenderPreset,
};
use clap::{Args, Subcommand};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Claber {
	/// Spawn a Claber quadruped through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = GenderPreset::Neutral)]
	pub gender: GenderPreset,

	#[arg(long, value_enum, default_value_t = BuildPreset::Average)]
	pub build: BuildPreset,

	#[arg(long, value_enum, default_value_t = ClaberHornMesh::HarrowedCrown)]
	pub horns: ClaberHornMesh,

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

impl Claber {
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
		let sliders = ClaberSliders::default()
			.with_shoulder_width(self.shoulder_width)
			.with_hip_width(self.hip_width)
			.with_chest_thickness(self.chest_thickness);
		ConceptPreviewConfig::claber_with_animation(
			ClaberConfig {
				gender: self.gender,
				build: self.build,
				horns: self.horns,
				eye: self.eye,
				colors: Default::default(),
				sliders,
			},
			self.animation,
		)
	}
}
