//! `/show kitchen-examples` — gallery of Kitchen cells with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct KitchenExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl KitchenExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::KitchenExamples, self.transform.transform())
	}
}
