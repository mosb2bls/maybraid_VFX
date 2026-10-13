//! Tapp color palettes — cooler, leaner Topple pastels.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TappPlumageColor {
	#[default]
	Mist,
	Pearl,
	SlateMist,
	SoftSage,
	CoolLavender,
}

impl TappPlumageColor {
	pub const VALUES: &'static [Self] =
		&[Self::Mist, Self::Pearl, Self::SlateMist, Self::SoftSage, Self::CoolLavender];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Mist => "mist",
			Self::Pearl => "pearl",
			Self::SlateMist => "slate-mist",
			Self::SoftSage => "soft-sage",
			Self::CoolLavender => "cool-lavender",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Mist => "#D4DCE8",
			Self::Pearl => "#E8E8F0",
			Self::SlateMist => "#C0C8D4",
			Self::SoftSage => "#C8D4C8",
			Self::CoolLavender => "#D0C8E0",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Mist => bevy::prelude::Color::srgb(0.83, 0.86, 0.91),
			Self::Pearl => bevy::prelude::Color::srgb(0.91, 0.91, 0.94),
			Self::SlateMist => bevy::prelude::Color::srgb(0.75, 0.78, 0.83),
			Self::SoftSage => bevy::prelude::Color::srgb(0.78, 0.83, 0.78),
			Self::CoolLavender => bevy::prelude::Color::srgb(0.82, 0.78, 0.88),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TappEyeColor {
	#[default]
	SoftBlue,
	SoftAmber,
	SoftRose,
}

impl TappEyeColor {
	pub const VALUES: &'static [Self] = &[Self::SoftBlue, Self::SoftAmber, Self::SoftRose];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftBlue => "soft-blue",
			Self::SoftAmber => "soft-amber",
			Self::SoftRose => "soft-rose",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::SoftBlue => "#8AB4D4",
			Self::SoftAmber => "#D4A86A",
			Self::SoftRose => "#D4A0B0",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftBlue => bevy::prelude::Color::srgb(0.54, 0.71, 0.83),
			Self::SoftAmber => bevy::prelude::Color::srgb(0.83, 0.66, 0.42),
			Self::SoftRose => bevy::prelude::Color::srgb(0.83, 0.63, 0.69),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TappBeakColor {
	#[default]
	Slate,
	Pearl,
	SoftPeach,
}

impl TappBeakColor {
	pub const VALUES: &'static [Self] = &[Self::Slate, Self::Pearl, Self::SoftPeach];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Slate => "slate",
			Self::Pearl => "pearl",
			Self::SoftPeach => "soft-peach",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Slate => "#A0A8B0",
			Self::Pearl => "#E0E0E8",
			Self::SoftPeach => "#E0C0A8",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Slate => bevy::prelude::Color::srgb(0.63, 0.66, 0.69),
			Self::Pearl => bevy::prelude::Color::srgb(0.88, 0.88, 0.91),
			Self::SoftPeach => bevy::prelude::Color::srgb(0.88, 0.75, 0.66),
		}
	}
}
