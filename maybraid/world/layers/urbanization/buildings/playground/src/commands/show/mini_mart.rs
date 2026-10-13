//! `/show mini-mart`

use bevy::prelude::*;
use clap::Args;

use super::bites_stall::BitesDoorSideArg;
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::{BitesDoorSide, PreviewSubject};

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct MiniMart {
	/// Stall AABB size `x,y,z` (min at origin). Needs room for 4×4 aisles + office.
	#[arg(long, default_value = "14,3.2,12", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	#[arg(long, default_value_t = 11)]
	pub seed: i32,
	/// Cardinal façade that receives demo Passage(s).
	#[arg(long, value_enum, default_value_t = BitesDoorSideArg::South)]
	pub door_side: BitesDoorSideArg,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl MiniMart {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::MiniMart {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				door_side: BitesDoorSide::from(self.door_side),
			},
			self.transform.transform(),
		)
	}
}
