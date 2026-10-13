//! `/show bites-sitdown-stall`

use bevy::prelude::*;
use clap::Args;

use super::bites_stall::BitesDoorSideArg;
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BitesSitdownStall {
	/// Stall AABB size `x,y,z` (min at origin).
	#[arg(long, default_value = "12,3.2,8", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	/// Cardinal façade that receives demo long Passage(s).
	#[arg(long, value_enum, default_value_t = BitesDoorSideArg::South)]
	pub door_side: BitesDoorSideArg,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl BitesSitdownStall {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::BitesSitdownStall {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				door_side: self.door_side.into(),
			},
			self.transform.transform(),
		)
	}
}
