//! `/show residential-bathroom-examples` — gallery of full + half residential bathrooms.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ResidentialBathroomExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ResidentialBathroomExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::ResidentialBathroomExamples, self.transform.transform())
	}
}
