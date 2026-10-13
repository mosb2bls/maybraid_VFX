//! `/show tessellated-triangle-3d`

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TessellatedTriangle3d {
	/// Corner A in world `x,y,z`.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a: Vec3,
	/// Corner B in world `x,y,z`.
	#[arg(long, default_value = "3,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b: Vec3,
	/// Corner C in world `x,y,z`.
	#[arg(long, default_value = "0,2,1", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub c: Vec3,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl TessellatedTriangle3d {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::TessellatedTriangle3d { a: self.a, b: self.b, c: self.c },
			self.transform.transform(),
		)
	}
}
