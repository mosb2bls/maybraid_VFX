//! `/show quad-panel-complex` — quad-face mesh (`{a0,a1,b0,b1}`) → panels + joints.
//!
//! Mesh syntax: `id=(x,y,z)[,…] ... {a0,a1,b0,b1}[,…]`
//! Diagonal is \(a_0\)–\(b_1\) (1st and 4th ids). Optional thickness: `id=(x,y,z,t)`.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

/// Default: mild kinked trapezoid as one quad.
const DEFAULT_MESH: &str = "1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,3,4}";

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct QuadPanelComplex {
	/// Compact quad mesh: `1=(x,y,z),2=… ... {1,2,3,4}` (optional 4th = thickness).
	#[arg(long, default_value = DEFAULT_MESH)]
	pub mesh: String,
	/// Spawn crease joint when dihedral kink (radians) is ≥ this threshold.
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	/// Force-omit crease joints (overrides `--min-dihedral`).
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl QuadPanelComplex {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::QuadPanelComplex {
				mesh: self.mesh,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
			},
			self.transform.transform(),
		)
	}
}
