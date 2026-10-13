//! `/show ruled-pitch` — [`RuledPitch`](buildings::RuledPitch) (eave/ridge
//! over [`RuledStrip`](buildings::RuledStrip)) + crease joints.
//!
//! Default: funky 5+5 — snaking eave near the ground, lagging ridge above.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RuledPitch {
	/// Spawn crease joint when dihedral kink (radians) is ≥ this threshold.
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	/// Force-omit crease joints (overrides `--min-dihedral`).
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RuledPitch {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::RuledPitch { min_dihedral: self.min_dihedral, no_joint: self.no_joint },
			self.transform.transform(),
		)
	}
}
