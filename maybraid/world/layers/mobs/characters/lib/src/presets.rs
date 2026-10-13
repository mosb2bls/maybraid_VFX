//! Shared preset identifiers for concept-stage character resolution.
//!
//! Presets are not restrictions. They initialize or refine resolved values before
//! command/UI fields apply their final overrides.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

// Shared vocabulary for every species in the concepts pass. Species modules may
// later restrict which variants they expose, but the IDs stay stable for CLI/UI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum GenderPreset {
	#[default]
	Neutral,
	Male,
	Female,
	NonBinary,
}

impl GenderPreset {
	pub const VALUES: &'static [Self] = &[Self::Neutral, Self::Male, Self::Female, Self::NonBinary];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Neutral => "neutral",
			Self::Male => "male",
			Self::Female => "female",
			Self::NonBinary => "non-binary",
		}
	}
}

// Applied after gender defaults in resolution order; neither preset narrows slider
// ranges—they only seed or refine values the user can still override.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BuildPreset {
	#[default]
	Average,
	Slender,
	Athletic,
	Heavy,
	Stocky,
	Lanky,
}

impl BuildPreset {
	pub const VALUES: &'static [Self] =
		&[Self::Average, Self::Slender, Self::Athletic, Self::Heavy, Self::Stocky, Self::Lanky];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Average => "average",
			Self::Slender => "slender",
			Self::Athletic => "athletic",
			Self::Heavy => "heavy",
			Self::Stocky => "stocky",
			Self::Lanky => "lanky",
		}
	}
}
