//! `/show connecting-stairwell-examples` — pathological exclusive-well stairwells.

use bevy::prelude::*;
use clap::Args;

use super::connecting_stairwell::StairwellFit;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingStairwellExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
	/// Circular helix, wall-hugging rectangular, or half-well I + landing.
	#[arg(long, value_enum, default_value_t = StairwellFit::Circular)]
	pub kind: StairwellFit,
}

impl ConnectingStairwellExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ConnectingStairwellExamples { kind: self.kind },
			self.transform.transform(),
		)
	}
}
