//! `/show wizards-tower`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct WizardsTower {
	/// Unit noise sample in \[0, 1\] for floor count (10..=30).
	#[arg(long, default_value_t = 0.5)]
	pub noise: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl WizardsTower {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::WizardsTower { noise: self.noise }, self.transform.transform())
	}
}
