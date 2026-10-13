//! Grener body palette — cool shark greys.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum GrenerBodyColor {
	#[default]
	Slate,
	Steel,
	Ink,
	Sand,
}

impl GrenerBodyColor {
	pub const VALUES: &'static [Self] = &[Self::Slate, Self::Steel, Self::Ink, Self::Sand];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Slate => "slate",
			Self::Steel => "steel",
			Self::Ink => "ink",
			Self::Sand => "sand",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Slate => "#6B7580",
			Self::Steel => "#8A96A3",
			Self::Ink => "#2C343C",
			Self::Sand => "#C4B49A",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Slate => bevy::prelude::Color::srgb(0.42, 0.46, 0.50),
			Self::Steel => bevy::prelude::Color::srgb(0.54, 0.59, 0.64),
			Self::Ink => bevy::prelude::Color::srgb(0.17, 0.20, 0.24),
			Self::Sand => bevy::prelude::Color::srgb(0.77, 0.71, 0.60),
		}
	}
}
