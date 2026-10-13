//! `/show bedroom-examples` — gallery of CommonBedroom cells with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BedroomExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl BedroomExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::BedroomExamples, self.transform.transform())
	}
}
