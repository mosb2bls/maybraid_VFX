//! `/show stacked-rings`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct StackedRings {
	/// Number of circular wall storeys to stack.
	#[arg(long, default_value_t = 8)]
	pub floor_count: u32,
	/// Height of each storey in world units (maps kit \(Y \in [0, 1]\)).
	#[arg(long, default_value_t = 3.0)]
	pub floor_height: f32,
	/// Outer wall radius in world units (maps kit radius \(1\)).
	#[arg(long, default_value_t = 4.0)]
	pub radius: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl StackedRings {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::StackedRings {
				floor_count: self.floor_count,
				floor_height: self.floor_height,
				radius: self.radius,
			},
			self.transform.transform(),
		)
	}
}
