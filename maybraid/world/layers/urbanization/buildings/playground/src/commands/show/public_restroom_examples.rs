//! `/show public-restroom-examples` — gallery of PublicRestroom stalls with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PublicRestroomExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl PublicRestroomExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::PublicRestroomExamples, self.transform.transform())
	}
}
