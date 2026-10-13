//! `/show livable-apartments-examples` — gallery of LivableApartments packs.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LivableApartmentsExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LivableApartmentsExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LivableApartmentsExamples, self.transform.transform())
	}
}
