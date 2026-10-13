//! `/show connecting-hall` — one-kink tube between two oriented openings.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingHall {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ConnectingHall {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::ConnectingHall, self.transform.transform())
	}
}
