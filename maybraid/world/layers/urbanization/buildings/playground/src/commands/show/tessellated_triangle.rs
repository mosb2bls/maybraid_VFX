//! `/show tessellated-triangle`

use bevy::prelude::*;
use bevy_math::Vec2;
use clap::Args;

use super::transform::parse_vec2_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TessellatedTriangle {
	/// Corner A in panel \(X,Z\) as `x,z` (negatives ok, e.g. `-1,0`).
	#[arg(long, default_value = "0,0", value_parser = parse_vec2_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Z")]
	pub a: Vec2,
	/// Corner B in panel \(X,Z\) as `x,z`.
	#[arg(long, default_value = "2,0", value_parser = parse_vec2_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Z")]
	pub b: Vec2,
	/// Corner C in panel \(X,Z\) as `x,z`.
	#[arg(long, default_value = "0,2", value_parser = parse_vec2_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Z")]
	pub c: Vec2,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl TessellatedTriangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::TessellatedTriangle { a: self.a, b: self.b, c: self.c },
			self.transform.transform(),
		)
	}
}
