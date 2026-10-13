//! `/show bites-stall`

use bevy::prelude::*;
use clap::Args;
use clap::ValueEnum;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::{BitesDoorSide, PreviewSubject};

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BitesStall {
	/// Stall AABB size `x,y,z` (min at origin).
	#[arg(long, default_value = "10,3.2,6", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum BitesDoorSideArg {
	South,
	North,
	East,
	West,
}

impl From<BitesDoorSideArg> for BitesDoorSide {
	fn from(value: BitesDoorSideArg) -> Self {
		match value {
			BitesDoorSideArg::South => Self::South,
			BitesDoorSideArg::North => Self::North,
			BitesDoorSideArg::East => Self::East,
			BitesDoorSideArg::West => Self::West,
		}
	}
}

impl BitesStall {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::BitesStall {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				door_side: self.door_side.into(),
			},
			self.transform.transform(),
		)
	}
}
