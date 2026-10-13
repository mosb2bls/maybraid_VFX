//! `/show fitted-rectangle` — single best-fit [`buildings::FittedRectangle`] kit.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum FittedRectanglePreset {
	/// Ground bay in XZ (`a` along +X, `b` along +Z).
	Floor,
	/// Standing bay in XY (`a` along +X, `b` along +Y).
	Wall,
	/// Ceiling bay in XZ at y=2.
	Ceiling,
	/// Skew bay (non-planar authored corners → planar fit).
	Skew,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct FittedRectangle {
	/// Convenience corners; overrides `--a0`/`--a1`/`--b0`/`--b1` when set.
	#[arg(long, value_enum)]
	pub preset: Option<FittedRectanglePreset>,
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a0: Vec3,
	#[arg(long, default_value = "3,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a1: Vec3,
	#[arg(long, default_value = "0,0,2", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b0: Vec3,
	#[arg(long, default_value = "3,0,2", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b1: Vec3,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl FittedRectangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		let (a0, a1, b0, b1) = match self.preset {
			Some(FittedRectanglePreset::Floor) => (
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(3.0, 0.0, 0.0),
				Vec3::new(0.0, 0.0, 2.0),
				Vec3::new(3.0, 0.0, 2.0),
			),
			Some(FittedRectanglePreset::Wall) => (
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.0),
				Vec3::new(0.0, 3.0, 0.0),
				Vec3::new(4.0, 3.0, 0.0),
			),
			Some(FittedRectanglePreset::Ceiling) => (
				Vec3::new(0.0, 2.0, 0.0),
				Vec3::new(3.0, 2.0, 0.0),
				Vec3::new(0.0, 2.0, 2.0),
				Vec3::new(3.0, 2.0, 2.0),
			),
			Some(FittedRectanglePreset::Skew) => (
				Vec3::ZERO,
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(0.1, 0.2, 1.0),
				Vec3::new(2.2, -0.1, 1.1),
			),
			None => (self.a0, self.a1, self.b0, self.b1),
		};
		(PreviewSubject::FittedRectangle { a0, a1, b0, b1 }, self.transform.transform())
	}
}
