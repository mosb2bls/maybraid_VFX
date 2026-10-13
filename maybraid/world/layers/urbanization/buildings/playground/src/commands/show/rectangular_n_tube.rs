//! `/show rectangular-n-tube` — closed n-gon cross-section polyline → n clipped rect strips.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectangularNTube {
	/// Uniform inset on face 1 middle bay (`0` = solid).
	#[arg(long, default_value_t = 0.35)]
	pub inset: f32,
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	/// Omit face on cross-section edge `i` (`i` → `i+1`). Repeatable; square demo: `0` floor, `2` ceiling.
	#[arg(long = "omit-face", value_name = "I")]
	pub omit_faces: Vec<usize>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RectangularNTube {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::RectangularNTube {
				inset: self.inset,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
				omit_faces: self.omit_faces,
			},
			self.transform.transform(),
		)
	}
}
