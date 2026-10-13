//! `/show linear`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Linear {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Linear {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Linear, self.transform.transform())
	}
}
