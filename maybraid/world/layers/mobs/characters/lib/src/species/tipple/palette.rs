//! Tipple color palettes — bright tweetie accents.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TipplePlumageColor {
	#[default]
	Yellow,
	Sky,
	Cherry,
	White,
}

impl TipplePlumageColor {
	pub const VALUES: &'static [Self] = &[Self::Yellow, Self::Sky, Self::Cherry, Self::White];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Yellow => "yellow",
			Self::Sky => "sky",
			Self::Cherry => "cherry",
			Self::White => "white",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Yellow => "#FFE14A",
			Self::Sky => "#4AB8FF",
			Self::Cherry => "#FF3A5A",
			Self::White => "#F5F5F0",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Yellow => bevy::prelude::Color::srgb(1.0, 0.88, 0.29),
			Self::Sky => bevy::prelude::Color::srgb(0.29, 0.72, 1.0),
			Self::Cherry => bevy::prelude::Color::srgb(1.0, 0.23, 0.35),
			Self::White => bevy::prelude::Color::srgb(0.96, 0.96, 0.94),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TippleEyeColor {
	#[default]
	Sky,
	Lemon,
	HotPink,
}

impl TippleEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Sky, Self::Lemon, Self::HotPink];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Sky => "sky",
			Self::Lemon => "lemon",
			Self::HotPink => "hot-pink",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Sky => "#2EC8E0",
			Self::Lemon => "#FFE14A",
			Self::HotPink => "#FF4FA3",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Sky => bevy::prelude::Color::srgb(0.18, 0.78, 0.88),
			Self::Lemon => bevy::prelude::Color::srgb(1.0, 0.88, 0.29),
			Self::HotPink => bevy::prelude::Color::srgb(1.0, 0.31, 0.64),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TippleBeakColor {
	#[default]
	Orange,
	Pink,
	Coral,
}

impl TippleBeakColor {
	pub const VALUES: &'static [Self] = &[Self::Orange, Self::Pink, Self::Coral];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Orange => "orange",
			Self::Pink => "pink",
			Self::Coral => "coral",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Orange => "#FF8A1A",
			Self::Pink => "#FF7AB8",
			Self::Coral => "#FF5C6B",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Orange => bevy::prelude::Color::srgb(1.0, 0.54, 0.10),
			Self::Pink => bevy::prelude::Color::srgb(1.0, 0.48, 0.72),
			Self::Coral => bevy::prelude::Color::srgb(1.0, 0.36, 0.42),
		}
	}
}
