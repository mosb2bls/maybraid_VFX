//! `/show tube` — trapezoid cross-section polyline → four clipped ruled strips.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Tube {
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	/// Omit the floor face from presentation.
	#[arg(long, default_value_t = false)]
	pub no_floor: bool,
	/// Omit the ceiling face from presentation.
	#[arg(long, default_value_t = false)]
	pub no_ceiling: bool,
	/// Omit the left wall from presentation.
	#[arg(long, default_value_t = false)]
	pub no_left: bool,
	/// Omit the right wall from presentation.
	#[arg(long, default_value_t = false)]
	pub no_right: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Tube {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::Tube {
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
				no_floor: self.no_floor,
				no_ceiling: self.no_ceiling,
				no_left: self.no_left,
				no_right: self.no_right,
			},
			self.transform.transform(),
		)
	}
}
