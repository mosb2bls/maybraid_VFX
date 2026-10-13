//! `/show les-halles-livable-full-storey-examples` — gallery of livable Les Halles storeys.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LesHallesLivableFullStoreyExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LesHallesLivableFullStoreyExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LesHallesLivableFullStoreyExamples, self.transform.transform())
	}
}
