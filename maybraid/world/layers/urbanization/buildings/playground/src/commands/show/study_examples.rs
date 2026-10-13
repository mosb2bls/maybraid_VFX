//! `/show study-examples` — gallery of Study cells with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct StudyExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl StudyExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::StudyExamples, self.transform.transform())
	}
}
