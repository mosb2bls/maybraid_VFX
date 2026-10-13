//! Mistler body palette — bright reef accents.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum MistlerBodyColor {
	#[default]
	Coral,
	Aqua,
	Lemon,
	Violet,
}

impl MistlerBodyColor {
	pub const VALUES: &'static [Self] = &[Self::Coral, Self::Aqua, Self::Lemon, Self::Violet];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Coral => "coral",
			Self::Aqua => "aqua",
			Self::Lemon => "lemon",
			Self::Violet => "violet",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Coral => "#FF6B5A",
			Self::Aqua => "#3AD4C8",
			Self::Lemon => "#FFE14A",
			Self::Violet => "#A46BFF",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Coral => bevy::prelude::Color::srgb(1.0, 0.42, 0.35),
			Self::Aqua => bevy::prelude::Color::srgb(0.23, 0.83, 0.78),
			Self::Lemon => bevy::prelude::Color::srgb(1.0, 0.88, 0.29),
			Self::Violet => bevy::prelude::Color::srgb(0.64, 0.42, 1.0),
		}
	}
}
