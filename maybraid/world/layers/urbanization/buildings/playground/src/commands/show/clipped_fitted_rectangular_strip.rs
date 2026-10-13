//! `/show clipped-fitted-rectangular-strip` — two-rail best-fit strip with a mid-bay inset.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedFittedRectangularStrip {
	#[arg(long, default_value_t = 0.35)]
	pub inset: f32,
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ClippedFittedRectangularStrip {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedFittedRectangularStrip {
				inset: self.inset,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
			},
			self.transform.transform(),
		)
	}
}
