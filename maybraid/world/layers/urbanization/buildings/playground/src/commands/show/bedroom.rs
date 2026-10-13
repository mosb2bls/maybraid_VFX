//! `/show bedroom` — [`buildings::CommonBedroom`] usage area.

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Bedroom {
	/// Cell AABB size `x,y,z` in world units (origin at min corner).
	#[arg(long, default_value = "7,3,7", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	/// Unit noise sample in \[0, 1\] for layout fitting.
	#[arg(long, default_value_t = 0.5)]
	pub noise: f32,
	/// Scale on each concept's base footprint (`1.0` = nominal).
	#[arg(long, default_value_t = 1.25)]
	pub spaciousness: f32,
	/// Max floor-area fraction to allocate (leave about `1 - occupancy` empty).
	#[arg(long, default_value_t = 0.55)]
	pub occupancy: f32,
	/// Punch a south (−Z) passage so entry clearance is reserved.
	#[arg(long, default_value_t = true)]
	pub door: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Bedroom {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::Bedroom {
				extent: self.extent.max(Vec3::splat(1e-4)),
				noise: self.noise.clamp(0.0, 1.0),
				spaciousness: self.spaciousness.max(1e-3),
				occupancy: self.occupancy.clamp(0.05, 1.0),
				door: self.door,
			},
			self.transform.transform(),
		)
	}
}
