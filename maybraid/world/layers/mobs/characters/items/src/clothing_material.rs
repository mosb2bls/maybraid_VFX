//! Clothing surface recipes resolved by character’s [`material_ref`] lib.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::clothing::ClothingMesh;

/// Named clothing looks. Palette[0] is the user color; the recipe is the shader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ClothingMaterial {
	SpaceSuit,
	Tattered,
	Hawaiian,
	#[default]
	Cloth,
	Scales,
	WizardsVeins,
	Glitter,
}

impl ClothingMaterial {
	pub const VALUES: &'static [Self] = &[
		Self::SpaceSuit,
		Self::Tattered,
		Self::Hawaiian,
		Self::Cloth,
		Self::Scales,
		Self::WizardsVeins,
		Self::Glitter,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SpaceSuit => "space-suit",
			Self::Tattered => "tattered",
			Self::Hawaiian => "hawaiian",
			Self::Cloth => "cloth",
			Self::Scales => "scales",
			Self::WizardsVeins => "wizards-veins",
			Self::Glitter => "glitter",
		}
	}

	/// [`material_ref::MaterialId::Name`] consumed by the character clothing lib.
	pub const fn recipe_id(self) -> &'static str {
		match self {
			Self::SpaceSuit => "clothing_space_suit",
			Self::Tattered => "clothing_tattered",
			Self::Hawaiian => "clothing_hawaiian",
			Self::Cloth => "clothing_cloth",
			Self::Scales => "clothing_scales",
			Self::WizardsVeins => "clothing_wizards_veins",
			Self::Glitter => "clothing_glitter",
		}
	}

	pub fn is_clothing_recipe(name: &str) -> bool {
		matches!(
			name,
			"clothing_space_suit"
				| "clothing_tattered"
				| "clothing_hawaiian"
				| "clothing_cloth"
				| "clothing_scales"
				| "clothing_wizards_veins"
				| "clothing_glitter"
		)
	}
}

/// Per-layer surface override; falls back to the default recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClothingMaterialChoice {
	pub clothing: ClothingMesh,
	pub material: ClothingMaterial,
}

impl ClothingMaterialChoice {
	pub fn resolve(
		overrides: &[ClothingMaterialChoice],
		default: ClothingMaterial,
		clothing: ClothingMesh,
	) -> ClothingMaterial {
		overrides
			.iter()
			.find(|choice| choice.clothing == clothing)
			.map(|choice| choice.material)
			.unwrap_or(default)
	}

	pub fn set(
		overrides: &mut Vec<ClothingMaterialChoice>,
		clothing: ClothingMesh,
		material: ClothingMaterial,
	) {
		if let Some(choice) = overrides.iter_mut().find(|choice| choice.clothing == clothing) {
			choice.material = material;
		} else {
			overrides.push(ClothingMaterialChoice { clothing, material });
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_look_has_a_recipe_id() {
		for material in ClothingMaterial::VALUES {
			assert!(ClothingMaterial::is_clothing_recipe(material.recipe_id()));
			assert!(!material.label().is_empty());
		}
	}

	#[test]
	fn per_item_choice_overrides_default() {
		use crate::clothing::ClothingMesh;

		let mut overrides = Vec::new();
		ClothingMaterialChoice::set(
			&mut overrides,
			ClothingMesh::Tunic,
			ClothingMaterial::Tattered,
		);
		assert_eq!(
			ClothingMaterialChoice::resolve(
				&overrides,
				ClothingMaterial::Cloth,
				ClothingMesh::Tunic
			),
			ClothingMaterial::Tattered
		);
		assert_eq!(
			ClothingMaterialChoice::resolve(
				&overrides,
				ClothingMaterial::Cloth,
				ClothingMesh::Pants
			),
			ClothingMaterial::Cloth
		);
	}
}
