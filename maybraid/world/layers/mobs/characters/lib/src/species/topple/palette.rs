//! Topple color palettes — soft pastels.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TopplePlumageColor {
	#[default]
	Cream,
	Blush,
	Powder,
	Sage,
	Lavender,
}

impl TopplePlumageColor {
	pub const VALUES: &'static [Self] =
		&[Self::Cream, Self::Blush, Self::Powder, Self::Sage, Self::Lavender];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Cream => "cream",
			Self::Blush => "blush",
			Self::Powder => "powder",
			Self::Sage => "sage",
			Self::Lavender => "lavender",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Cream => "#F3E8D4",
			Self::Blush => "#E8C4C4",
			Self::Powder => "#C4D4E8",
			Self::Sage => "#C4D4B8",
			Self::Lavender => "#D4C4E0",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Cream => bevy::prelude::Color::srgb(0.95, 0.91, 0.83),
			Self::Blush => bevy::prelude::Color::srgb(0.91, 0.77, 0.77),
			Self::Powder => bevy::prelude::Color::srgb(0.77, 0.83, 0.91),
			Self::Sage => bevy::prelude::Color::srgb(0.77, 0.83, 0.72),
			Self::Lavender => bevy::prelude::Color::srgb(0.83, 0.77, 0.88),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ToppleEyeColor {
	#[default]
	SoftAmber,
	SoftBlue,
	SoftRose,
}

impl ToppleEyeColor {
	pub const VALUES: &'static [Self] = &[Self::SoftAmber, Self::SoftBlue, Self::SoftRose];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftAmber => "soft-amber",
			Self::SoftBlue => "soft-blue",
			Self::SoftRose => "soft-rose",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::SoftAmber => "#D4A86A",
			Self::SoftBlue => "#8AB4D4",
			Self::SoftRose => "#D4A0B0",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftAmber => bevy::prelude::Color::srgb(0.83, 0.66, 0.42),
			Self::SoftBlue => bevy::prelude::Color::srgb(0.54, 0.71, 0.83),
			Self::SoftRose => bevy::prelude::Color::srgb(0.83, 0.63, 0.69),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ToppleBeakColor {
	#[default]
	Peach,
	Cream,
	Blush,
}

impl ToppleBeakColor {
	pub const VALUES: &'static [Self] = &[Self::Peach, Self::Cream, Self::Blush];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Peach => "peach",
			Self::Cream => "cream",
			Self::Blush => "blush",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Peach => "#E8B890",
			Self::Cream => "#F0E0C8",
			Self::Blush => "#E8A8A8",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Peach => bevy::prelude::Color::srgb(0.91, 0.72, 0.56),
			Self::Cream => bevy::prelude::Color::srgb(0.94, 0.88, 0.78),
			Self::Blush => bevy::prelude::Color::srgb(0.91, 0.66, 0.66),
		}
	}
}
