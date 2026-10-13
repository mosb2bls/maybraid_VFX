//! `/show clipped-arc-sweep` — circular [`ClippedArcSweep`] with angular openings.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedArcSweep {
	#[arg(long, default_value_t = 4.0)]
	pub radius: f32,
	#[arg(long, default_value_t = 3.0)]
	pub height: f32,
	#[arg(long, default_value_t = 180.0)]
	pub sweep_degrees: f32,
	#[arg(long, default_value_t = 0.0)]
	pub start_yaw_deg: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ClippedArcSweep {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedArcSweep {
				radius: self.radius,
				height: self.height,
				sweep_degrees: self.sweep_degrees,
				start_yaw_deg: self.start_yaw_deg,
			},
			self.transform.transform(),
		)
	}
}
