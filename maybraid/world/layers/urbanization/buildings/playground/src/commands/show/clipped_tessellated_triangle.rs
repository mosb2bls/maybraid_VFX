//! `/show clipped-tessellated-triangle` — outer triangle minus a closed clip polyline.

use bevy::prelude::*;
use clap::Args;

use super::transform::{parse_vec3_csv, parse_vec3_polyline, Vec3Polyline};
use super::ShowTransform;
use crate::preview::PreviewSubject;

/// Default: ground △ + small rectangle near the centroid.
const DEFAULT_CLIP: &str = "0.8,0,0.5;1.4,0,0.5;1.4,0,0.9;0.8,0,0.9";

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedTessellatedTriangle {
	/// Outer corner A in world `x,y,z`.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a: Vec3,
	/// Outer corner B in world `x,y,z`.
	#[arg(long, default_value = "3,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b: Vec3,
	/// Outer corner C in world `x,y,z`.
	#[arg(long, default_value = "0,0,2", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub c: Vec3,
	/// Closed clip polyline: `x,y,z;x,y,z;…` (first connects to last; may extend outside).
	#[arg(long, default_value = DEFAULT_CLIP, value_parser = parse_vec3_polyline, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z;…")]
	pub clip: Vec3Polyline,
	/// Spawn crease joint when dihedral kink (radians) is ≥ this threshold.
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	/// Force-omit crease joints (overrides `--min-dihedral`).
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ClippedTessellatedTriangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedTessellatedTriangle {
				a: self.a,
				b: self.b,
				c: self.c,
				clip: self.clip.0,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
			},
			self.transform.transform(),
		)
	}
}
