//! `/show i-apartment-full-storey-examples` — gallery of full storeys with LivableApartments.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct IApartmentFullStoreyExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl IApartmentFullStoreyExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::IApartmentFullStoreyExamples, self.transform.transform())
	}
}
