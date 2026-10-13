//! `/show pitch`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Pitch {
	/// Rise from eave to ridge (pitch-space Y). Non-negative.
	#[arg(long, default_value_t = 1.0)]
	pub rise: f32,
	/// Run from eave to ridge (pitch-space Z). Non-negative.
	#[arg(long, default_value_t = 2.0)]
	pub run: f32,
	/// Rectangular span along X. Omit for ends-only.
	#[arg(long)]
	pub length: Option<f32>,
	/// Suggested tile width along X (fitted to `length`).
	#[arg(long, default_value_t = 1.0)]
	pub tile_width: f32,
	/// Left end-triangle base length. Positive = upright, negative = flipped. Omit for none.
	#[arg(long, allow_negative_numbers = true)]
	pub left: Option<f32>,
	/// Right end-triangle base length. Positive = upright, negative = flipped. Omit for none.
	#[arg(long, allow_negative_numbers = true)]
	pub right: Option<f32>,
	/// Alternative to `--length/--left/--right`: build via equal end triangles from eave & ridge.
	#[arg(long)]
	pub eave: Option<f32>,
	/// Used with `--eave` for [`Pitch::from_eave_ridge`].
	#[arg(long)]
	pub ridge: Option<f32>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Pitch {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		let (length, left, right) = if let (Some(eave), Some(ridge)) = (self.eave, self.ridge) {
			let p = building_components::roofs::Pitch::from_eave_ridge(
				self.rise,
				self.run,
				eave,
				ridge,
				self.tile_width,
			);
			(p.length, p.left, p.right)
		} else {
			(self.length, self.left, self.right)
		};
		(
			PreviewSubject::Pitch {
				rise: self.rise,
				run: self.run,
				length,
				tile_width: self.tile_width,
				left,
				right,
			},
			self.transform.transform(),
		)
	}
}
