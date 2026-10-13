//! `/show arc-180`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Arc180 {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Arc180 {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Arc180, self.transform.transform())
	}
}
