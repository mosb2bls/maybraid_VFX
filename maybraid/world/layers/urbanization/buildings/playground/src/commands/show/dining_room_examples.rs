//! `/show dining-room-examples` — gallery of DiningRoom cells with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct DiningRoomExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl DiningRoomExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::DiningRoomExamples, self.transform.transform())
	}
}
