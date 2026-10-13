//! `/show knick-knack-examples` — gallery of KnickKnack stalls with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct KnickKnackExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl KnickKnackExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::KnickKnackExamples, self.transform.transform())
	}
}
