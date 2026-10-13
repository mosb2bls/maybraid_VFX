//! `/show noisy-rectangular-wall` — noisy path → [`NoisyRectangularWall`] panel strip.

use bevy::prelude::*;
use clap::Args;
use procedural_common::{AllowedAngles, NoiseParams, StepLenRange};

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct NoisyRectangularWall {
	/// Total path length budget.
	#[arg(long, default_value_t = 12.0)]
	pub distance: f32,
	/// Minimum segment length per step.
	#[arg(long, default_value_t = 0.75)]
	pub step_len_min: f32,
	/// Maximum segment length per step.
	#[arg(long, default_value_t = 1.25)]
	pub step_len_max: f32,
	/// Max absolute pitch from horizontal (radians) — the vertical angle.
	#[arg(long, default_value_t = 0.4)]
	pub max_angle_x: f32,
	/// Max per-step yaw (radians about world +Y).
	#[arg(long, default_value_t = 0.55)]
	pub max_angle_y: f32,
	/// Max per-step roll (radians; unused for path positions).
	#[arg(long, default_value_t = 0.0)]
	pub max_angle_z: f32,
	/// Path noise seed.
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl NoisyRectangularWall {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::NoisyRectangularWall {
				distance: self.distance.max(1e-3),
				step_len: StepLenRange::new(self.step_len_min, self.step_len_max),
				allowed_angles: AllowedAngles::new(
					self.max_angle_x,
					self.max_angle_y,
					self.max_angle_z,
				),
				path_noise: NoiseParams {
					seed: self.seed,
					frequency: 0.35,
					..NoiseParams::default()
				},
			},
			self.transform.transform(),
		)
	}
}
