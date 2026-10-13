//! `/show rectangle` — single oriented [`buildings::Rectangle`] kit.

use bevy::prelude::*;
use clap::{Args, ValueEnum};
use std::f32::consts::{FRAC_PI_2, PI};

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum RectanglePreset {
	/// Ground bay in XZ (edge along +Z, roll −π/2 so height → +X).
	Floor,
	/// Standing bay in XY (edge along +X, roll 0 so height → +Y).
	Wall,
	/// Ceiling bay in XZ at y=2 (edge along +X, roll π so height → −Y).
	Ceiling,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Rectangle {
	/// Convenience orientation; overrides origin/edge/height/roll when set.
	#[arg(long, value_enum)]
	pub preset: Option<RectanglePreset>,
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub origin: Vec3,
	/// Lowest-edge path (length = `|edge|`).
	#[arg(long, default_value = "0,0,2", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub edge: Vec3,
	#[arg(long, default_value_t = 3.0)]
	pub height: f32,
	#[arg(long, default_value_t = 0.75)]
	pub thickness: f32,
	/// `0` ⇒ top toward world `+Y`.
	#[arg(long, default_value_t = 0.0)]
	pub roll: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Rectangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		let (origin, edge, height, thickness, roll) = match self.preset {
			Some(RectanglePreset::Floor) => {
				(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), 3.0, 0.75, -FRAC_PI_2)
			}
			Some(RectanglePreset::Wall) => (Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), 3.0, 0.75, 0.0),
			Some(RectanglePreset::Ceiling) => {
				(Vec3::new(0.0, 2.0, 0.0), Vec3::new(3.0, 0.0, 0.0), 2.0, 0.75, PI)
			}
			None => (self.origin, self.edge, self.height, self.thickness, self.roll),
		};
		(
			PreviewSubject::Rectangle { origin, edge, height, thickness, roll },
			self.transform.transform(),
		)
	}
}
