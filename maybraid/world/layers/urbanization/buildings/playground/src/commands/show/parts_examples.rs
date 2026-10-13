//! `/show parts-examples` — gallery of Parts stalls with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PartsExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl PartsExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::PartsExamples, self.transform.transform())
	}
}
