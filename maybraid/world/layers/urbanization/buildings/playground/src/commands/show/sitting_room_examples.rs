//! `/show sitting-room-examples` — gallery of SittingRoom cells with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct SittingRoomExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl SittingRoomExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::SittingRoomExamples, self.transform.transform())
	}
}
