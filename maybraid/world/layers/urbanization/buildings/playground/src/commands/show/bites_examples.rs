//! `/show bites-examples` — gallery of bites + sit-down stalls with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BitesExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl BitesExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::BitesExamples, self.transform.transform())
	}
}
