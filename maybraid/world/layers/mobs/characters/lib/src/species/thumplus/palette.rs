//! Thumplus body palette — deep whale blues.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ThumplusBodyColor {
	#[default]
	Ocean,
	Midnight,
	Fog,
	Ivory,
}

impl ThumplusBodyColor {
	pub const VALUES: &'static [Self] = &[Self::Ocean, Self::Midnight, Self::Fog, Self::Ivory];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ocean => "ocean",
			Self::Midnight => "midnight",
			Self::Fog => "fog",
			Self::Ivory => "ivory",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Ocean => "#3A5F7A",
			Self::Midnight => "#1A2838",
			Self::Fog => "#8A9AAA",
			Self::Ivory => "#E8E4D8",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ocean => bevy::prelude::Color::srgb(0.23, 0.37, 0.48),
			Self::Midnight => bevy::prelude::Color::srgb(0.10, 0.16, 0.22),
			Self::Fog => bevy::prelude::Color::srgb(0.54, 0.60, 0.67),
			Self::Ivory => bevy::prelude::Color::srgb(0.91, 0.89, 0.85),
		}
	}
}
