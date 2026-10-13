//! `/show arc-tower` — stacked circular storey shell (no noise).

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ArcTower {
	#[arg(long, default_value_t = 4.0)]
	pub radius: f32,
	#[arg(long, default_value_t = 3)]
	pub floor_count: u32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Centered square hole side length on intermediate floors (`0` = solid).
	#[arg(long, default_value_t = 2.24)]
	pub floor_hole: f32,
	#[arg(long, default_value_t = false)]
	pub no_base_floor: bool,
	#[arg(long, default_value_t = false)]
	pub no_ceiling: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ArcTower {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ArcTower {
				radius: self.radius,
				floor_count: self.floor_count,
				storey_height: self.storey_height,
				floor_hole: self.floor_hole,
				no_base_floor: self.no_base_floor,
				no_ceiling: self.no_ceiling,
			},
			self.transform.transform(),
		)
	}
}
