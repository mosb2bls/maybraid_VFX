//! Chupri color palettes — bright tropical accents for a tiny bird.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ChupriPlumageColor {
	#[default]
	Magenta,
	Cyan,
	Lime,
	Canary,
	Violet,
}

impl ChupriPlumageColor {
	pub const VALUES: &'static [Self] =
		&[Self::Magenta, Self::Cyan, Self::Lime, Self::Canary, Self::Violet];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Magenta => "magenta",
			Self::Cyan => "cyan",
			Self::Lime => "lime",
			Self::Canary => "canary",
			Self::Violet => "violet",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Magenta => "#E83A8A",
			Self::Cyan => "#2EC8E0",
			Self::Lime => "#7AE02E",
			Self::Canary => "#F5D024",
			Self::Violet => "#9B5CFF",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Magenta => bevy::prelude::Color::srgb(0.91, 0.23, 0.54),
			Self::Cyan => bevy::prelude::Color::srgb(0.18, 0.78, 0.88),
			Self::Lime => bevy::prelude::Color::srgb(0.48, 0.88, 0.18),
			Self::Canary => bevy::prelude::Color::srgb(0.96, 0.82, 0.14),
			Self::Violet => bevy::prelude::Color::srgb(0.61, 0.36, 1.0),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ChupriEyeColor {
	#[default]
	Turquoise,
	Lemon,
	HotPink,
}

impl ChupriEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Turquoise, Self::Lemon, Self::HotPink];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Turquoise => "turquoise",
			Self::Lemon => "lemon",
			Self::HotPink => "hot-pink",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Turquoise => "#1AD4C8",
			Self::Lemon => "#FFE14A",
			Self::HotPink => "#FF4FA3",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Turquoise => bevy::prelude::Color::srgb(0.10, 0.83, 0.78),
			Self::Lemon => bevy::prelude::Color::srgb(1.0, 0.88, 0.29),
			Self::HotPink => bevy::prelude::Color::srgb(1.0, 0.31, 0.64),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ChupriBeakColor {
	#[default]
	Tangerine,
	Coral,
	Chartreuse,
}

impl ChupriBeakColor {
	pub const VALUES: &'static [Self] = &[Self::Tangerine, Self::Coral, Self::Chartreuse];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Tangerine => "tangerine",
			Self::Coral => "coral",
			Self::Chartreuse => "chartreuse",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Tangerine => "#FF8A1A",
			Self::Coral => "#FF5C6B",
			Self::Chartreuse => "#C6FF2E",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Tangerine => bevy::prelude::Color::srgb(1.0, 0.54, 0.10),
			Self::Coral => bevy::prelude::Color::srgb(1.0, 0.36, 0.42),
			Self::Chartreuse => bevy::prelude::Color::srgb(0.78, 1.0, 0.18),
		}
	}
}
