//! `/show i-apartment-floor-plan-examples` — Fit gallery of I-frame floor plans.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct IApartmentFloorPlanExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl IApartmentFloorPlanExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::IApartmentFloorPlanExamples, self.transform.transform())
	}
}
