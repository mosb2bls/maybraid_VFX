//! Persistable create-mode appearance. Clothing lives on the inventory bag.

use character_items::{ClothingColor, ClothingMaterialChoice, Inventory, InventoryItem};
use serde::{Deserialize, Serialize};

use crate::species::{
	braidman::BraidmanConfig, brodler::BrodlerConfig, dui::DuiConfig, lero::LeroConfig,
	mygr::MygrConfig, spibmom::SpibmomConfig, tuberwaber::TuberwaberConfig, wumbus::WumbusConfig,
};

/// Humanoid appearance written to `characters/{id}.json`. Worn garments are not
/// stored here; they come from the matching inventory file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "species", content = "config", rename_all = "kebab-case")]
pub enum CharacterAppearance {
	Braidman(BraidmanConfig),
	Brodler(BrodlerConfig),
	Mygr(MygrConfig),
	Dui(DuiConfig),
	Wumbus(WumbusConfig),
	Lero(LeroConfig),
	Spibmom(SpibmomConfig),
	Tuberwaber(TuberwaberConfig),
}

impl Default for CharacterAppearance {
	fn default() -> Self {
		Self::Braidman(BraidmanConfig::default_preview())
	}
}

impl CharacterAppearance {
	pub fn species_id(&self) -> &'static str {
		match self {
			Self::Braidman(_) => "braidman",
			Self::Brodler(_) => "brodler",
			Self::Mygr(_) => "mygr",
			Self::Dui(_) => "dui",
			Self::Wumbus(_) => "wumbus",
			Self::Lero(_) => "lero",
			Self::Spibmom(_) => "spibmom",
			Self::Tuberwaber(_) => "tuberwaber",
		}
	}

	pub fn species_title(&self) -> &'static str {
		match self {
			Self::Braidman(_) => "Braidman",
			Self::Brodler(_) => "Brodler",
			Self::Mygr(_) => "Mygr",
			Self::Dui(_) => "Dui",
			Self::Wumbus(_) => "Wumbus",
			Self::Lero(_) => "Lero",
			Self::Spibmom(_) => "Spibmom",
			Self::Tuberwaber(_) => "Tuberwaber",
		}
	}

	/// Drop clothing layers so the model file does not duplicate the bag.
	pub fn strip_clothing(&mut self) {
		match self {
			Self::Braidman(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Brodler(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Mygr(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Dui(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Wumbus(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Lero(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Spibmom(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Tuberwaber(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
		}
	}

	/// Rebuild the rendered garment layers from the inventory's worn selection.
	pub fn with_inventory_clothing(mut self, inventory: &Inventory) -> Self {
		let clothing: Vec<_> = inventory.worn_items().filter_map(InventoryItem::mesh).collect();
		let colors: Vec<_> = inventory
			.worn_items()
			.filter_map(|item| {
				item.mesh()
					.zip(item.material())
					.map(|(clothing, material)| ClothingColor { clothing, color: material.color })
			})
			.collect();
		let materials: Vec<_> = inventory
			.worn_items()
			.filter_map(|item| {
				item.mesh().zip(item.material()).map(|(clothing, material)| {
					ClothingMaterialChoice { clothing, material: material.id }
				})
			})
			.collect();
		macro_rules! apply {
			($config:expr) => {{
				$config.clothing = clothing;
				$config.colors.clothing = colors;
				$config.colors.clothing_materials = materials;
			}};
		}
		match &mut self {
			Self::Braidman(config) => apply!(config),
			Self::Brodler(config) => apply!(config),
			Self::Mygr(config) => apply!(config),
			Self::Dui(config) => apply!(config),
			Self::Wumbus(config) => apply!(config),
			Self::Lero(config) => apply!(config),
			Self::Spibmom(config) => apply!(config),
			Self::Tuberwaber(config) => apply!(config),
		}
		self
	}
}

fn strip_humanoid_clothing(
	clothing: &mut Vec<character_items::ClothingMesh>,
	colors: &mut Vec<character_items::ClothingColor>,
	materials: &mut Vec<character_items::ClothingMaterialChoice>,
) {
	clothing.clear();
	colors.clear();
	materials.clear();
}

#[cfg(test)]
mod tests {
	use anyhow::Context;
	use character_items::{ClothingMaterial, ClothingMesh, Inventory, InventoryItem, ItemColor};

	use crate::appearance::CharacterAppearance;

	#[test]
	fn inventory_clothing_rebuilds_the_rendered_loadout() -> anyhow::Result<()> {
		let worn = InventoryItem::clothing(
			ClothingMesh::Tunic,
			ClothingMaterial::Glitter,
			ItemColor::Blue,
		);
		let packed =
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red);
		let inventory = Inventory {
			items: vec![worn, packed],
			clothing: vec![0],
			weapons: Vec::new(),
			skills: Vec::new(),
		};
		let appearance = CharacterAppearance::default().with_inventory_clothing(&inventory);
		let CharacterAppearance::Braidman(config) = appearance else {
			anyhow::bail!("default appearance should remain a braidman");
		};
		let color = config.colors.clothing.first().context("expected a worn color")?;
		let material =
			config.colors.clothing_materials.first().context("expected a worn material")?;
		assert_eq!(config.clothing, vec![ClothingMesh::Tunic]);
		assert_eq!(color.color, ItemColor::Blue);
		assert_eq!(material.material, ClothingMaterial::Glitter);
		Ok(())
	}
}
