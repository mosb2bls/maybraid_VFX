//! Spibmom color palettes.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomSkinColor {
	#[default]
	PowderBlue,
	MistBlue,
	SoftSky,
	PalePeriwinkle,
	DustyBlue,
}

impl SpibmomSkinColor {
	pub const VALUES: &'static [Self] =
		&[Self::PowderBlue, Self::MistBlue, Self::SoftSky, Self::PalePeriwinkle, Self::DustyBlue];

	pub const fn label(self) -> &'static str {
		match self {
			Self::PowderBlue => "powder-blue",
			Self::MistBlue => "mist-blue",
			Self::SoftSky => "soft-sky",
			Self::PalePeriwinkle => "pale-periwinkle",
			Self::DustyBlue => "dusty-blue",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::PowderBlue => "#B8CCE0",
			Self::MistBlue => "#A6BDD1",
			Self::SoftSky => "#94ADC7",
			Self::PalePeriwinkle => "#B3B8DB",
			Self::DustyBlue => "#8C9EB8",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::PowderBlue => bevy::prelude::Color::srgb(0.72, 0.80, 0.88),
			Self::MistBlue => bevy::prelude::Color::srgb(0.65, 0.74, 0.82),
			Self::SoftSky => bevy::prelude::Color::srgb(0.58, 0.68, 0.78),
			Self::PalePeriwinkle => bevy::prelude::Color::srgb(0.70, 0.72, 0.86),
			Self::DustyBlue => bevy::prelude::Color::srgb(0.55, 0.62, 0.72),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomEyeColor {
	#[default]
	Pearl,
	Frost,
	Ivory,
}

impl SpibmomEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Pearl, Self::Frost, Self::Ivory];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pearl => "pearl",
			Self::Frost => "frost",
			Self::Ivory => "ivory",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Pearl => "#F5F5F0",
			Self::Frost => "#F0F5FA",
			Self::Ivory => "#F5F0E5",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Pearl => bevy::prelude::Color::srgb(0.96, 0.96, 0.94),
			Self::Frost => bevy::prelude::Color::srgb(0.94, 0.96, 0.98),
			Self::Ivory => bevy::prelude::Color::srgb(0.96, 0.94, 0.90),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomEarColor {
	#[default]
	Umber,
	Charcoal,
	DeepSlate,
}

impl SpibmomEarColor {
	pub const VALUES: &'static [Self] = &[Self::Umber, Self::Charcoal, Self::DeepSlate];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Umber => "umber",
			Self::Charcoal => "charcoal",
			Self::DeepSlate => "deep-slate",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Umber => "#523D2E",
			Self::Charcoal => "#3D3833",
			Self::DeepSlate => "#333D47",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Umber => bevy::prelude::Color::srgb(0.32, 0.24, 0.18),
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.22, 0.20),
			Self::DeepSlate => bevy::prelude::Color::srgb(0.20, 0.24, 0.28),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomMouthColor {
	#[default]
	Espresso,
	Charcoal,
	DeepSlate,
}

impl SpibmomMouthColor {
	pub const VALUES: &'static [Self] = &[Self::Espresso, Self::Charcoal, Self::DeepSlate];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Espresso => "espresso",
			Self::Charcoal => "charcoal",
			Self::DeepSlate => "deep-slate",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Espresso => "#382924",
			Self::Charcoal => "#333338",
			Self::DeepSlate => "#2E3842",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Espresso => bevy::prelude::Color::srgb(0.22, 0.16, 0.14),
			Self::Charcoal => bevy::prelude::Color::srgb(0.20, 0.20, 0.22),
			Self::DeepSlate => bevy::prelude::Color::srgb(0.18, 0.22, 0.26),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomCrownColor {
	#[default]
	Charcoal,
	DeepBronze,
	DarkUmber,
}

impl SpibmomCrownColor {
	pub const VALUES: &'static [Self] = &[Self::Charcoal, Self::DeepBronze, Self::DarkUmber];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Charcoal => "charcoal",
			Self::DeepBronze => "deep-bronze",
			Self::DarkUmber => "dark-umber",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Charcoal => "#3D3D42",
			Self::DeepBronze => "#57422E",
			Self::DarkUmber => "#4D3829",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.24, 0.26),
			Self::DeepBronze => bevy::prelude::Color::srgb(0.34, 0.26, 0.18),
			Self::DarkUmber => bevy::prelude::Color::srgb(0.30, 0.22, 0.16),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomSpineColor {
	#[default]
	Charcoal,
	DeepBronze,
	DarkUmber,
}

impl SpibmomSpineColor {
	pub const VALUES: &'static [Self] = &[Self::Charcoal, Self::DeepBronze, Self::DarkUmber];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Charcoal => "charcoal",
			Self::DeepBronze => "deep-bronze",
			Self::DarkUmber => "dark-umber",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Charcoal => "#3D3D42",
			Self::DeepBronze => "#57422E",
			Self::DarkUmber => "#4D3829",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.24, 0.26),
			Self::DeepBronze => bevy::prelude::Color::srgb(0.34, 0.26, 0.18),
			Self::DarkUmber => bevy::prelude::Color::srgb(0.30, 0.22, 0.16),
		}
	}
}
