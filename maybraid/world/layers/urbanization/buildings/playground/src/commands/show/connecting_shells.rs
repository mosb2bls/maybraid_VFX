//! `/show connecting-shells` — ArcTower + ConnectingHall + Trazaloid demo.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingShells {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ConnectingShells {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::ConnectingShells, self.transform.transform())
	}
}
