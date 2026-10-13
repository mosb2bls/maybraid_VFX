//! `/show les-halles-floor-plan-examples` — gallery of Les Halles floor plans.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LesHallesFloorPlanExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LesHallesFloorPlanExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LesHallesFloorPlanExamples, self.transform.transform())
	}
}
