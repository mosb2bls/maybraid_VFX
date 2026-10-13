//! Lidder color palettes.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LidderPlumageColor {
	#[default]
	Slate,
	Ash,
	Sand,
	Ink,
}

impl LidderPlumageColor {
	pub const VALUES: &'static [Self] = &[Self::Slate, Self::Ash, Self::Sand, Self::Ink];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Slate => "slate",
			Self::Ash => "ash",
			Self::Sand => "sand",
			Self::Ink => "ink",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Slate => "#6E7A85",
			Self::Ash => "#9A9590",
			Self::Sand => "#B8A58A",
			Self::Ink => "#2C3038",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Slate => bevy::prelude::Color::srgb(0.43, 0.48, 0.52),
			Self::Ash => bevy::prelude::Color::srgb(0.60, 0.58, 0.56),
			Self::Sand => bevy::prelude::Color::srgb(0.72, 0.65, 0.54),
			Self::Ink => bevy::prelude::Color::srgb(0.17, 0.19, 0.22),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LidderEyeColor {
	#[default]
	Amber,
	Gold,
	Dark,
}

impl LidderEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Amber, Self::Gold, Self::Dark];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Amber => "amber",
			Self::Gold => "gold",
			Self::Dark => "dark",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Amber => "#C48A2E",
			Self::Gold => "#D4B04A",
			Self::Dark => "#1A1A1E",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Amber => bevy::prelude::Color::srgb(0.77, 0.54, 0.18),
			Self::Gold => bevy::prelude::Color::srgb(0.83, 0.69, 0.29),
			Self::Dark => bevy::prelude::Color::srgb(0.10, 0.10, 0.12),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LidderBeakColor {
	#[default]
	Horn,
	Coral,
	Charcoal,
}

impl LidderBeakColor {
	pub const VALUES: &'static [Self] = &[Self::Horn, Self::Coral, Self::Charcoal];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Horn => "horn",
			Self::Coral => "coral",
			Self::Charcoal => "charcoal",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Horn => "#C4A06A",
			Self::Coral => "#C46A5A",
			Self::Charcoal => "#3A3A40",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Horn => bevy::prelude::Color::srgb(0.77, 0.63, 0.42),
			Self::Coral => bevy::prelude::Color::srgb(0.77, 0.42, 0.35),
			Self::Charcoal => bevy::prelude::Color::srgb(0.23, 0.23, 0.25),
		}
	}
}
