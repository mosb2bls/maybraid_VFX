//! Firearm surface and bolt looks.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Named firearm surface recipes. Veins / glitter / scales share clothing
/// shader looks; lava veins and brushed metal are firearm-only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmMaterial {
	WizardsVeins,
	Glitter,
	Scales,
	LavaVeins,
	#[default]
	BrushedMetal,
}

impl FirearmMaterial {
	pub const VALUES: &'static [Self] =
		&[Self::WizardsVeins, Self::Glitter, Self::Scales, Self::LavaVeins, Self::BrushedMetal];

	pub const fn label(self) -> &'static str {
		match self {
			Self::WizardsVeins => "wizards-veins",
			Self::Glitter => "glitter",
			Self::Scales => "scales",
			Self::LavaVeins => "lava-veins",
			Self::BrushedMetal => "brushed-metal",
		}
	}

	/// [`material_ref::MaterialId::Name`] consumed by the character clothing lib.
	pub const fn recipe_id(self) -> &'static str {
		match self {
			Self::WizardsVeins => "firearm_wizards_veins",
			Self::Glitter => "firearm_glitter",
			Self::Scales => "firearm_scales",
			Self::LavaVeins => "firearm_lava_veins",
			Self::BrushedMetal => "firearm_brushed_metal",
		}
	}

	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::WizardsVeins => &["Gerund", "Starmage", "Sigil"],
			Self::Glitter => &["Beacon", "Clastappal", "Sequined", "Shimmering"],
			Self::Scales => &["Plated", "Oblisk", "Cylint"],
			Self::LavaVeins => &["Furnaceheld", "'tilinforge", "Pelely"],
			Self::BrushedMetal => &["Brushed", "Service", "Issue", "Machined", "Filed"],
		}
	}

	pub fn is_firearm_recipe(name: &str) -> bool {
		matches!(
			name,
			"firearm_wizards_veins"
				| "firearm_glitter"
				| "firearm_scales"
				| "firearm_lava_veins"
				| "firearm_brushed_metal"
		)
	}
}

/// Projectile / beam look. Stats contribute through buff-deviation; spawn
/// still uses the default glow until combat wiring lands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BoltMaterial {
	#[default]
	PlainLaser,
	FizzingLaser,
}

impl BoltMaterial {
	pub const VALUES: &'static [Self] = &[Self::PlainLaser, Self::FizzingLaser];

	pub const fn label(self) -> &'static str {
		match self {
			Self::PlainLaser => "plain-laser",
			Self::FizzingLaser => "fizzing-laser",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_firearm_look_has_a_recipe_id() {
		for material in FirearmMaterial::VALUES {
			assert!(FirearmMaterial::is_firearm_recipe(material.recipe_id()));
			assert!(!material.label().is_empty());
			assert!(!material.adjectives().is_empty());
		}
	}
}
