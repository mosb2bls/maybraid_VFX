//! `/show livable-rectangles-examples` — gallery of RectangularLivableArea fits.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LivableRectanglesExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LivableRectanglesExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LivableRectanglesExamples, self.transform.transform())
	}
}
