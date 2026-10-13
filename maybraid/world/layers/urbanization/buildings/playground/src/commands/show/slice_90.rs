//! `/show slice-90`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Slice90 {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Slice90 {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Slice90, self.transform.transform())
	}
}
