//! `/show clipped-fitted-rectangle` — best-fit rectangle with an inset framed by rectangle kits.

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedFittedRectangle {
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a0: Vec3,
	#[arg(long, default_value = "2,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a1: Vec3,
	#[arg(long, default_value = "0.1,0.2,1", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b0: Vec3,
	#[arg(long, default_value = "2.2,-0.1,1.1", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b1: Vec3,
	#[arg(long, default_value_t = 0.3)]
	pub left: f32,
	#[arg(long, default_value_t = 0.3)]
	pub right: f32,
	#[arg(long, default_value_t = 0.2)]
	pub bottom: f32,
	#[arg(long, default_value_t = 0.2)]
	pub top: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ClippedFittedRectangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedFittedRectangle {
				a0: self.a0,
				a1: self.a1,
				b0: self.b0,
				b1: self.b1,
				left: self.left,
				right: self.right,
				bottom: self.bottom,
				top: self.top,
			},
			self.transform.transform(),
		)
	}
}
