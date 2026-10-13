//! `/show polyline` — `Partition::polyline` kit posing (linears + joints).

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Polyline {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Polyline {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Polyline, self.transform.transform())
	}
}
