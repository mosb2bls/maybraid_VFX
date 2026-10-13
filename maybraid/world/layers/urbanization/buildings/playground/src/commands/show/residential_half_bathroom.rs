//! `/show residential-half-bathroom` — [`buildings::ResidentialHalfBathroom`] usage area.

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ResidentialHalfBathroom {
	/// Cell AABB size `x,y,z` in world units (origin at min corner).
	#[arg(long, default_value = "1.8,2.8,1.5", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	#[arg(long, default_value_t = 7)]
	pub seed: i32,
	/// Punch a south (−Z) passage so entry clearance is reserved.
	#[arg(long, default_value_t = true)]
	pub door: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ResidentialHalfBathroom {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ResidentialHalfBathroom {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				door: self.door,
			},
			self.transform.transform(),
		)
	}
}
