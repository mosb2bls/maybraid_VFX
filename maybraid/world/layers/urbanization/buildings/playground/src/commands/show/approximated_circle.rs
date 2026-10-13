//! `/show approximated-circle` — n-gon disk / annulus via paneling.

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ApproximatedCircle {
	/// Disk center in world `x,y,z`.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub center: Vec3,
	#[arg(long, default_value_t = 3.0)]
	pub radius: f32,
	#[arg(long, default_value_t = 24)]
	pub segments: u32,
	/// Concentric hole radius (omit / 0 for a solid disk).
	#[arg(long, default_value_t = 0.8)]
	pub clip: f32,
	/// When set, ignore `--clip` and build a solid disk.
	#[arg(long, default_value_t = false)]
	pub solid: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ApproximatedCircle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		let clip = if self.solid || self.clip <= 1e-6 { None } else { Some(self.clip) };
		(
			PreviewSubject::ApproximatedCircle {
				center: self.center,
				radius: self.radius,
				segments: self.segments,
				clip,
			},
			self.transform.transform(),
		)
	}
}
