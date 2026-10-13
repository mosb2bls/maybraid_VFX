//! `/show arc-90`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Arc90 {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Arc90 {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Arc90, self.transform.transform())
	}
}
