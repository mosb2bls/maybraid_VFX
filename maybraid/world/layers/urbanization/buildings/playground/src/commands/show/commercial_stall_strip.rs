//! `/show commercial-stall-strip`

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct CommercialStallStrip {
	/// Strip AABB size `x,y,z` (min at origin; long axis preferred on X).
	#[arg(long, default_value = "14,3.5,5", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl CommercialStallStrip {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::CommercialStallStrip {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
			},
			self.transform.transform(),
		)
	}
}
