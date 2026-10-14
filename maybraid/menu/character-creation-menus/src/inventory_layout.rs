//! Shared inventory layout: Clothing, Tools, Skill Maps, Lubikan.
//!
//! Character generation and the character-menu Inventory modal lower to the
//! same [`MenuNode`] tree. Generation is [`InventoryLayoutMode::ReadOnly`]
//! (view + Continue). The modal is [`InventoryLayoutMode::Interactive`]
//! (equip + drop). Tools is a UI grouping; the domain slot stays Weapons.

use character_items::{Inventory, InventoryItem, InventorySlot, WORN_CLOTHING_LIMIT};
use character_ui_menu::{AssetOption, GridCatalogChoice, MenuNode, PreviewColor, ThumbnailCamera};

use crate::event::MenuEvent;

pub const INVENTORY_LABEL: &str = "Inventory";
pub const STARTER_KIT_LABEL: &str = "Starter Kit";
pub const TOOLS_LABEL: &str = "Tools";
pub const LUBIKAN_LABEL: &str = "Lubikan";

/// Equip / drop policy for one inventory layout instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryLayoutMode {
	/// Character-menu modal: tiles toggle wear / queue and can drop.
	Interactive,
	/// Character-generation Starter Kit: view only.
	ReadOnly,
}

impl InventoryLayoutMode {
	pub const fn allows_equip(self) -> bool {
		matches!(self, Self::Interactive)
	}

	pub const fn allows_drop(self) -> bool {
		matches!(self, Self::Interactive)
	}

	pub const fn allows(self, event: MenuEvent) -> bool {
		match event {
			MenuEvent::ToggleInventory(_) => self.allows_equip(),
			MenuEvent::DropInventory(_) => self.allows_drop(),
			_ => false,
		}
	}
}

/// One sectioned inventory tree plus the chrome title and interactivity.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryLayout {
	pub title: &'static str,
	pub mode: InventoryLayoutMode,
	pub node: MenuNode<MenuEvent>,
}

impl InventoryLayout {
	/// Read-only grant screen. Title is [`STARTER_KIT_LABEL`].
	pub fn starter_kit(items: impl Into<Vec<InventoryItem>>) -> Self {
		let inventory = Inventory::with_starter_outfit(items.into());
		Self::from_inventory(STARTER_KIT_LABEL, InventoryLayoutMode::ReadOnly, &inventory)
	}

	/// Interactive character-menu overlay. Title is [`INVENTORY_LABEL`].
	pub fn character_menu(inventory: &Inventory) -> Self {
		Self::from_inventory(INVENTORY_LABEL, InventoryLayoutMode::Interactive, inventory)
	}

	pub fn from_inventory(
		title: &'static str,
		mode: InventoryLayoutMode,
		inventory: &Inventory,
	) -> Self {
		Self { title, mode, node: inventory_node(title, mode, inventory) }
	}

	pub fn allows(&self, event: MenuEvent) -> bool {
		self.mode.allows(event)
	}

	/// Clothing / Tools / Skill Maps / Lubikan in that order.
	pub fn section_labels(&self) -> Vec<&'static str> {
		section_labels(&self.node)
	}
}

fn inventory_node(
	title: &'static str,
	mode: InventoryLayoutMode,
	inventory: &Inventory,
) -> MenuNode<MenuEvent> {
	MenuNode::section(
		title,
		MenuNode::fragment([
			MenuNode::section("Clothing", clothing_catalog(mode, inventory)),
			MenuNode::section(TOOLS_LABEL, tools_catalog(mode, inventory)),
			MenuNode::section("Skill Maps", skills_catalog(mode, inventory)),
			lubikan_section(),
		]),
	)
}

fn catalog_event(_mode: InventoryLayoutMode, index: usize) -> MenuEvent {
	// Index is still the bag slot so hosts can identify the tile. Read-only
	// mode rejects apply of both Toggle and Drop; the renderer omits the
	// activate extras.
	MenuEvent::ToggleInventory(index)
}

fn clothing_catalog(mode: InventoryLayoutMode, inventory: &Inventory) -> MenuNode<MenuEvent> {
	MenuNode::grid_catalog(
		"Clothing",
		WORN_CLOTHING_LIMIT,
		inventory.items.iter().enumerate().filter_map(|(index, item)| {
			let mesh = item.mesh()?;
			let asset = mesh.asset();
			let rank = inventory.rank(index);
			Some(GridCatalogChoice {
				label: item.name(),
				detail: item.catalog_detail(),
				path: asset.path,
				thumbnail_camera: asset.thumbnail_camera,
				preview: PreviewColor::of(item.material()?.color),
				image_key: item.icon_catalog_key(),
				selected: rank.is_some(),
				rank,
				event: catalog_event(mode, index),
			})
		}),
	)
}

fn tools_catalog(mode: InventoryLayoutMode, inventory: &Inventory) -> MenuNode<MenuEvent> {
	MenuNode::grid_catalog(
		TOOLS_LABEL,
		InventorySlot::Weapons.capacity(),
		inventory.items.iter().enumerate().filter_map(|(index, item)| {
			if item.firearm_mesh().is_none() && item.grenade_spec().is_none() {
				return None;
			}
			let rank = inventory.rank(index);
			let (path, thumbnail_camera) = match item.firearm_mesh() {
				Some(mesh) => {
					let asset = mesh.asset();
					(asset.path, asset.thumbnail_camera)
				}
				None => (item.path(), ThumbnailCamera::DEFAULT),
			};
			Some(GridCatalogChoice {
				label: item.name(),
				detail: item.catalog_detail(),
				path,
				thumbnail_camera,
				preview: PreviewColor::WHITE,
				image_key: item.icon_catalog_key(),
				selected: rank.is_some(),
				rank,
				event: catalog_event(mode, index),
			})
		}),
	)
}

fn skills_catalog(mode: InventoryLayoutMode, inventory: &Inventory) -> MenuNode<MenuEvent> {
	MenuNode::grid_catalog(
		InventorySlot::Skills.label(),
		InventorySlot::Skills.capacity(),
		inventory.items.iter().enumerate().filter_map(|(index, item)| {
			let spec = item.skill_map_spec()?;
			let [red, green, blue] = spec.kind.preview_srgb();
			let rank = inventory.rank(index);
			Some(GridCatalogChoice {
				label: item.name(),
				detail: item.catalog_detail(),
				path: item.path(),
				thumbnail_camera: ThumbnailCamera::DEFAULT,
				preview: PreviewColor::srgb(red, green, blue),
				image_key: item.icon_catalog_key(),
				selected: rank.is_some(),
				rank,
				event: catalog_event(mode, index),
			})
		}),
	)
}

fn lubikan_section() -> MenuNode<MenuEvent> {
	MenuNode::section(LUBIKAN_LABEL, MenuNode::labeled_value("Self", "Always equipped"))
}

fn section_labels(node: &MenuNode<MenuEvent>) -> Vec<&'static str> {
	let MenuNode::Section { children, .. } = node else {
		return Vec::new();
	};
	children
		.iter()
		.filter_map(|child| match child {
			MenuNode::Section { label, .. } => Some(*label),
			_ => None,
		})
		.collect()
}

/// Bag large enough that Clothing + Tools + Skill Maps overflow a short pane.
pub fn overflow_inventory_items() -> Vec<InventoryItem> {
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, ItemColor, SkillMapKind, SkillMapSpec,
	};

	let mut items: Vec<InventoryItem> = ClothingMesh::VALUES
		.iter()
		.copied()
		.map(|mesh| InventoryItem::clothing(mesh, ClothingMaterial::Cloth, ItemColor::Natural))
		.collect();
	items.extend(FirearmMesh::VALUES.iter().copied().map(InventoryItem::firearm));
	items.push(InventoryItem::standard_grenade());
	items.push(InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)));
	items.push(InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)));
	items
}

#[cfg(test)]
mod tests {
	use super::{
		overflow_inventory_items, InventoryLayout, InventoryLayoutMode, STARTER_KIT_LABEL,
	};
	use crate::MenuEvent;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	};
	use character_ui_menu::MenuNode;

	fn sample_items() -> Vec<InventoryItem> {
		vec![
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red),
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::standard_grenade(),
		]
	}

	#[test]
	fn both_instances_share_the_four_section_headers() {
		let items = sample_items();
		let starter = InventoryLayout::starter_kit(items.clone());
		let menu = InventoryLayout::character_menu(&Inventory::with_starter_outfit(items));
		assert_eq!(starter.section_labels(), ["Clothing", "Tools", "Skill Maps", "Lubikan"]);
		assert_eq!(starter.section_labels(), menu.section_labels());
		assert_eq!(starter.title, STARTER_KIT_LABEL);
		assert_eq!(menu.title, "Inventory");
		assert_eq!(starter.mode, InventoryLayoutMode::ReadOnly);
		assert_eq!(menu.mode, InventoryLayoutMode::Interactive);
	}

	#[test]
	fn read_only_mode_rejects_equip_and_drop() {
		let layout = InventoryLayout::starter_kit(sample_items());
		assert!(!layout.allows(MenuEvent::ToggleInventory(0)));
		assert!(!layout.allows(MenuEvent::DropInventory(0)));
		assert!(!layout.allows(MenuEvent::Save));
		assert!(!InventoryLayoutMode::ReadOnly.allows_equip());
		assert!(!InventoryLayoutMode::ReadOnly.allows_drop());
	}

	#[test]
	fn interactive_mode_allows_equip_and_drop() {
		let inventory = Inventory::with_starter_outfit(sample_items());
		let layout = InventoryLayout::character_menu(&inventory);
		assert!(layout.allows(MenuEvent::ToggleInventory(0)));
		assert!(layout.allows(MenuEvent::DropInventory(0)));
		assert!(InventoryLayoutMode::Interactive.allows_equip());
		assert!(InventoryLayoutMode::Interactive.allows_drop());
	}

	#[test]
	fn overflow_bag_has_more_tiles_than_a_short_viewport() {
		let items = overflow_inventory_items();
		let layout = InventoryLayout::starter_kit(items);
		let MenuNode::Section { children, .. } = &layout.node else {
			panic!("starter kit must be a section");
		};
		let tiles: usize = children
			.iter()
			.map(|child| match child {
				MenuNode::Section { children, .. } => children
					.iter()
					.map(|inner| match inner {
						MenuNode::GridCatalog { choices, .. } => choices.len(),
						_ => 0,
					})
					.sum(),
				_ => 0,
			})
			.sum();
		// 12 clothing + 5 firearms + 1 grenade + 2 skill maps. A 400px pane
		// cannot show ~20 catalog tiles plus four headers at once.
		assert!(tiles >= 18, "overflow bag should have many tiles, got {tiles}");
	}
}
