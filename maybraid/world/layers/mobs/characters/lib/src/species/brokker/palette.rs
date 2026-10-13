//! Brokker color palettes — muted reptilian / pterosaur tones.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrokkerPlumageColor {
	#[default]
	Olive,
	Ochre,
	Slate,
	Rust,
}

impl BrokkerPlumageColor {
	pub const VALUES: &'static [Self] = &[Self::Olive, Self::Ochre, Self::Slate, Self::Rust];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Olive => "olive",
			Self::Ochre => "ochre",
			Self::Slate => "slate",
			Self::Rust => "rust",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Olive => "#6B7A4A",
			Self::Ochre => "#A8874A",
			Self::Slate => "#6A737C",
			Self::Rust => "#8A5A3C",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Olive => bevy::prelude::Color::srgb(0.42, 0.48, 0.29),
			Self::Ochre => bevy::prelude::Color::srgb(0.66, 0.53, 0.29),
			Self::Slate => bevy::prelude::Color::srgb(0.42, 0.45, 0.49),
			Self::Rust => bevy::prelude::Color::srgb(0.54, 0.35, 0.24),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrokkerEyeColor {
	#[default]
	Amber,
	Gold,
	Dark,
}

impl BrokkerEyeColor {
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
pub enum BrokkerSnoutColor {
	#[default]
	Horn,
	Olive,
	Charcoal,
}

impl BrokkerSnoutColor {
	pub const VALUES: &'static [Self] = &[Self::Horn, Self::Olive, Self::Charcoal];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Horn => "horn",
			Self::Olive => "olive",
			Self::Charcoal => "charcoal",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Horn => "#C4A06A",
			Self::Olive => "#7A8A4A",
			Self::Charcoal => "#3A3A40",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Horn => bevy::prelude::Color::srgb(0.77, 0.63, 0.42),
			Self::Olive => bevy::prelude::Color::srgb(0.48, 0.54, 0.29),
			Self::Charcoal => bevy::prelude::Color::srgb(0.23, 0.23, 0.25),
		}
	}
}
