//! Kaller color palettes — reptilian olive / moss / scale tones.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerPlumageColor {
	#[default]
	Olive,
	ScaleGreen,
	Moss,
	Rust,
	Ochre,
}

impl KallerPlumageColor {
	pub const VALUES: &'static [Self] =
		&[Self::Olive, Self::ScaleGreen, Self::Moss, Self::Rust, Self::Ochre];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Olive => "olive",
			Self::ScaleGreen => "scale-green",
			Self::Moss => "moss",
			Self::Rust => "rust",
			Self::Ochre => "ochre",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Olive => "#6B7A4A",
			Self::ScaleGreen => "#5A7A52",
			Self::Moss => "#4A6A42",
			Self::Rust => "#8A5A3C",
			Self::Ochre => "#A8874A",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Olive => bevy::prelude::Color::srgb(0.42, 0.48, 0.29),
			Self::ScaleGreen => bevy::prelude::Color::srgb(0.35, 0.48, 0.32),
			Self::Moss => bevy::prelude::Color::srgb(0.29, 0.42, 0.26),
			Self::Rust => bevy::prelude::Color::srgb(0.54, 0.35, 0.24),
			Self::Ochre => bevy::prelude::Color::srgb(0.66, 0.53, 0.29),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerEyeColor {
	#[default]
	Amber,
	Gold,
	Slit,
}

impl KallerEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Amber, Self::Gold, Self::Slit];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Amber => "amber",
			Self::Gold => "gold",
			Self::Slit => "slit",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Amber => "#C48A2E",
			Self::Gold => "#D4B04A",
			Self::Slit => "#B8A030",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Amber => bevy::prelude::Color::srgb(0.77, 0.54, 0.18),
			Self::Gold => bevy::prelude::Color::srgb(0.83, 0.69, 0.29),
			Self::Slit => bevy::prelude::Color::srgb(0.72, 0.63, 0.19),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerSnoutColor {
	#[default]
	Horn,
	Olive,
	Charcoal,
}

impl KallerSnoutColor {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerCrownColor {
	#[default]
	Charcoal,
	Olive,
	Bone,
}

impl KallerCrownColor {
	pub const VALUES: &'static [Self] = &[Self::Charcoal, Self::Olive, Self::Bone];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Charcoal => "charcoal",
			Self::Olive => "olive",
			Self::Bone => "bone",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Charcoal => "#3D3D42",
			Self::Olive => "#6B7A4A",
			Self::Bone => "#E0D4C0",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.24, 0.26),
			Self::Olive => bevy::prelude::Color::srgb(0.42, 0.48, 0.29),
			Self::Bone => bevy::prelude::Color::srgb(0.88, 0.83, 0.75),
		}
	}
}
