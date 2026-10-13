//! `/show clipped-rectangle` — oriented rectangle with an inset framed by rectangle kits.

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedRectangle {
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub origin: Vec3,
	#[arg(long, default_value = "0,0,2", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub edge: Vec3,
	#[arg(long, default_value_t = 3.0)]
	pub height: f32,
	#[arg(long, default_value_t = 0.75)]
	pub thickness: f32,
	#[arg(long, default_value_t = 0.0)]
	pub roll: f32,
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

impl ClippedRectangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedRectangle {
				origin: self.origin,
				edge: self.edge,
				height: self.height,
				thickness: self.thickness,
				roll: self.roll,
				left: self.left,
				right: self.right,
				bottom: self.bottom,
				top: self.top,
			},
			self.transform.transform(),
		)
	}
}
