//! `/show livable-apartment-examples` — gallery of standalone LivableApartment layouts.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LivableApartmentExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LivableApartmentExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LivableApartmentExamples, self.transform.transform())
	}
}
