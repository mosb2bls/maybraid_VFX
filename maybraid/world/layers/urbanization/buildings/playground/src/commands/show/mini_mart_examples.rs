//! `/show mini-mart-examples` — gallery of MiniMart stalls with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct MiniMartExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl MiniMartExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::MiniMartExamples, self.transform.transform())
	}
}
