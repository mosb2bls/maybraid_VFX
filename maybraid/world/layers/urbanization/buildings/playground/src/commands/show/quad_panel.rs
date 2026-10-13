//! `/show quad-panel` — ruled quad: two tessellated triangles + optional crease joint.

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct QuadPanel {
	/// Line A start (`a0`) in world `x,y,z`.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a0: Vec3,
	/// Line A end (`a1`) in world `x,y,z`.
	#[arg(long, default_value = "3,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a1: Vec3,
	/// Line B start (`b0`) in world `x,y,z`.
	#[arg(long, default_value = "0,3,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b0: Vec3,
	/// Line B end (`b1`) in world `x,y,z`.
	#[arg(long, default_value = "0,0,3", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b1: Vec3,
	/// Panel thickness at `a0` (default matches unscaled panel kit = 0.4).
	#[arg(long, default_value_t = 0.4)]
	pub t_a0: f32,
	/// Panel thickness at `a1`.
	#[arg(long, default_value_t = 0.4)]
	pub t_a1: f32,
	/// Panel thickness at `b0`.
	#[arg(long, default_value_t = 0.4)]
	pub t_b0: f32,
	/// Panel thickness at `b1`.
	#[arg(long, default_value_t = 0.4)]
	pub t_b1: f32,
	/// Spawn crease joint when dihedral kink (radians) is ≥ this threshold.
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	/// Force-omit the crease joint (overrides `--min-dihedral`).
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl QuadPanel {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::QuadPanel {
				a0: self.a0,
				a1: self.a1,
				b0: self.b0,
				b1: self.b1,
				t_a0: self.t_a0,
				t_a1: self.t_a1,
				t_b0: self.t_b0,
				t_b1: self.t_b1,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
			},
			self.transform.transform(),
		)
	}
}
