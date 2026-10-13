//! Wumbus color palettes.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusSkinColor {
	#[default]
	Chocolate,
	Espresso,
	Umber,
	Soot,
}

impl WumbusSkinColor {
	pub const VALUES: &'static [Self] = &[Self::Chocolate, Self::Espresso, Self::Umber, Self::Soot];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Chocolate => "chocolate",
			Self::Espresso => "espresso",
			Self::Umber => "umber",
			Self::Soot => "soot",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Chocolate => "#473328",
			Self::Espresso => "#2E241F",
			Self::Umber => "#523D2E",
			Self::Soot => "#1F1C1A",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Chocolate => bevy::prelude::Color::srgb(0.28, 0.20, 0.16),
			Self::Espresso => bevy::prelude::Color::srgb(0.18, 0.14, 0.12),
			Self::Umber => bevy::prelude::Color::srgb(0.32, 0.24, 0.18),
			Self::Soot => bevy::prelude::Color::srgb(0.12, 0.11, 0.10),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusEyeColor {
	#[default]
	PaleBlue,
	Honey,
	Sage,
}

impl WumbusEyeColor {
	pub const VALUES: &'static [Self] = &[Self::PaleBlue, Self::Honey, Self::Sage];

	pub const fn label(self) -> &'static str {
		match self {
			Self::PaleBlue => "pale-blue",
			Self::Honey => "honey",
			Self::Sage => "sage",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::PaleBlue => "#9EB8C7",
			Self::Honey => "#C7AD6B",
			Self::Sage => "#94AD8C",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::PaleBlue => bevy::prelude::Color::srgb(0.62, 0.72, 0.78),
			Self::Honey => bevy::prelude::Color::srgb(0.78, 0.68, 0.42),
			Self::Sage => bevy::prelude::Color::srgb(0.58, 0.68, 0.55),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusEarColor {
	#[default]
	Cream,
	Sandy,
	RustTip,
}

impl WumbusEarColor {
	pub const VALUES: &'static [Self] = &[Self::Cream, Self::Sandy, Self::RustTip];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Cream => "cream",
			Self::Sandy => "sandy",
			Self::RustTip => "rust-tip",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Cream => "#E0D1B8",
			Self::Sandy => "#C7AD85",
			Self::RustTip => "#B87A52",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Cream => bevy::prelude::Color::srgb(0.88, 0.82, 0.72),
			Self::Sandy => bevy::prelude::Color::srgb(0.78, 0.68, 0.52),
			Self::RustTip => bevy::prelude::Color::srgb(0.72, 0.48, 0.32),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusMouthColor {
	#[default]
	Blush,
	DustyRose,
	PaleCoral,
}

impl WumbusMouthColor {
	pub const VALUES: &'static [Self] = &[Self::Blush, Self::DustyRose, Self::PaleCoral];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Blush => "blush",
			Self::DustyRose => "dusty-rose",
			Self::PaleCoral => "pale-coral",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Blush => "#D19E94",
			Self::DustyRose => "#BF8C85",
			Self::PaleCoral => "#E0A899",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Blush => bevy::prelude::Color::srgb(0.82, 0.62, 0.58),
			Self::DustyRose => bevy::prelude::Color::srgb(0.75, 0.55, 0.52),
			Self::PaleCoral => bevy::prelude::Color::srgb(0.88, 0.66, 0.60),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusHornColor {
	#[default]
	Ivory,
	Wheat,
	PaleGold,
}

impl WumbusHornColor {
	pub const VALUES: &'static [Self] = &[Self::Ivory, Self::Wheat, Self::PaleGold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ivory => "ivory",
			Self::Wheat => "wheat",
			Self::PaleGold => "pale-gold",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Ivory => "#E0D6BD",
			Self::Wheat => "#D1BD8F",
			Self::PaleGold => "#E5D19E",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ivory => bevy::prelude::Color::srgb(0.88, 0.84, 0.74),
			Self::Wheat => bevy::prelude::Color::srgb(0.82, 0.74, 0.56),
			Self::PaleGold => bevy::prelude::Color::srgb(0.90, 0.82, 0.62),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusSpineColor {
	#[default]
	Ivory,
	Wheat,
	PaleGold,
}

impl WumbusSpineColor {
	pub const VALUES: &'static [Self] = &[Self::Ivory, Self::Wheat, Self::PaleGold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ivory => "ivory",
			Self::Wheat => "wheat",
			Self::PaleGold => "pale-gold",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Ivory => "#E0D6BD",
			Self::Wheat => "#D1BD8F",
			Self::PaleGold => "#E5D19E",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ivory => bevy::prelude::Color::srgb(0.88, 0.84, 0.74),
			Self::Wheat => bevy::prelude::Color::srgb(0.82, 0.74, 0.56),
			Self::PaleGold => bevy::prelude::Color::srgb(0.90, 0.82, 0.62),
		}
	}
}
