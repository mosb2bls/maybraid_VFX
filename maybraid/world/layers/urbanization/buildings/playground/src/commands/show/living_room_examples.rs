//! `/show living-room-examples` — gallery of LivingRoom cells with passage gizmos.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LivingRoomExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LivingRoomExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LivingRoomExamples, self.transform.transform())
	}
}
