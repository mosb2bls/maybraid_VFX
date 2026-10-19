//! Shared inventory layout: Clothing, Tools, Skill Maps, Tanzes, Lubikan.
//!
//! Character generation and the character-menu Inventory modal lower to the
//! same [`MenuNode`] tree. Generation is [`InventoryLayoutMode::ReadOnly`]
//! (view + Continue). The modal is [`InventoryLayoutMode::Interactive`]
//! (equip + drop). Tools is a UI grouping; the domain slot stays Weapons.
//! Tanz tiles reuse pose-capture catalog keys from `tanz-user`.

use character_items::{
	icon_catalog_key_for_label, self_lubikan_catalog_key, Inventory, InventoryItem, InventorySlot,
	SELF_LUBIKAN_CAPTURE_LABEL, WORN_CLOTHING_LIMIT,
};
use character_ui_menu::{AssetOption, GridCatalogChoice, MenuNode, PreviewColor, ThumbnailCamera};
use std::time::Duration;
use tanz_user::TanzPose;

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

/// Scene-capture the layout waits on before painting tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryIconNeed {
	pub key: u64,
	pub silhouette: bool,
}

impl InventoryIconNeed {
	pub fn shaded(key: u64) -> Self {
		Self { key, silhouette: false }
	}

	pub fn silhouette(key: u64) -> Self {
		Self { key, silhouette: true }
	}
}

/// Whether the shared inventory should still show the loading wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryPaint {
	Loading,
	Ready,
}

/// Give up and paint placeholders rather than spin forever.
pub const INVENTORY_CAPTURE_TIMEOUT: Duration = Duration::from_secs(45);

/// Loading until every required capture reports filled or failed, or timeout.
pub fn inventory_paint_state(
	reported: impl IntoIterator<Item = bool>,
	waited: Duration,
	timeout: Duration,
) -> InventoryPaint {
	let reported: Vec<bool> = reported.into_iter().collect();
	if reported.iter().all(|&done| done) || waited >= timeout {
		InventoryPaint::Ready
	} else {
		InventoryPaint::Loading
	}
}

/// One sectioned inventory tree plus the chrome title and interactivity.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryLayout {
	pub title: &'static str,
	pub mode: InventoryLayoutMode,
	pub node: MenuNode<MenuEvent>,
	/// Clothing / tool / Self Lubikan captures. Skill maps are not scene-captured.
	pub captures: Vec<InventoryIconNeed>,
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
		Self {
			title,
			mode,
			node: inventory_node(title, inventory),
			captures: required_captures_for(inventory),
		}
	}

	pub fn allows(&self, event: MenuEvent) -> bool {
		self.mode.allows(event)
	}

	/// Clothing / Tools / Skill Maps / Tanzes / Lubikan in that order.
	pub fn section_labels(&self) -> Vec<&'static str> {
		section_labels(&self.node)
	}
}

fn required_captures_for(inventory: &Inventory) -> Vec<InventoryIconNeed> {
	let mut needs = vec![InventoryIconNeed::silhouette(self_lubikan_catalog_key())];
	for item in &inventory.items {
		if matches!(item, InventoryItem::SkillMap { .. }) {
			continue;
		}
		if let Some(id) = item.tanz_id() {
			needs.push(InventoryIconNeed::silhouette(TanzPose(id).icon_catalog_key()));
			continue;
		}
		if let Some(key) = item.icon_catalog_key() {
			needs.push(InventoryIconNeed::shaded(key));
		}
	}
	needs
}

fn inventory_node(title: &'static str, inventory: &Inventory) -> MenuNode<MenuEvent> {
	MenuNode::section(
		title,
		MenuNode::fragment([
			MenuNode::section("Clothing", clothing_catalog(inventory)),
			MenuNode::section(TOOLS_LABEL, tools_catalog(inventory)),
			MenuNode::section("Skill Maps", skills_catalog(inventory)),
			MenuNode::section(InventorySlot::Tanzes.label(), tanz_catalog(inventory)),
			lubikan_section(),
		]),
	)
}

fn catalog_event(index: usize) -> Option<MenuEvent> {
	Some(MenuEvent::ToggleInventory(index))
}

fn clothing_catalog(inventory: &Inventory) -> MenuNode<MenuEvent> {
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
				event: catalog_event(index),
			})
		}),
	)
}

fn tools_catalog(inventory: &Inventory) -> MenuNode<MenuEvent> {
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
				event: catalog_event(index),
			})
		}),
	)
}

fn skills_catalog(inventory: &Inventory) -> MenuNode<MenuEvent> {
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
				event: catalog_event(index),
			})
		}),
	)
}

fn tanz_catalog(inventory: &Inventory) -> MenuNode<MenuEvent> {
	MenuNode::grid_catalog(
		InventorySlot::Tanzes.label(),
		InventorySlot::Tanzes.capacity(),
		inventory.items.iter().enumerate().filter_map(|(index, item)| {
			let id = item.tanz_id()?;
			let rank = inventory.rank(index);
			Some(GridCatalogChoice {
				label: item.name(),
				detail: item.catalog_detail(),
				path: item.path(),
				thumbnail_camera: ThumbnailCamera::DEFAULT,
				preview: PreviewColor::WHITE,
				image_key: Some(TanzPose(id).icon_catalog_key()),
				selected: rank.is_some(),
				rank,
				event: catalog_event(index),
			})
		}),
	)
}

fn lubikan_section() -> MenuNode<MenuEvent> {
	MenuNode::section(
		LUBIKAN_LABEL,
		MenuNode::grid_catalog(
			LUBIKAN_LABEL,
			1,
			[GridCatalogChoice {
				label: String::from("Self"),
				detail: String::from("Always equipped"),
				path: SELF_LUBIKAN_CAPTURE_LABEL,
				thumbnail_camera: ThumbnailCamera::DEFAULT,
				preview: PreviewColor::WHITE,
				image_key: Some(icon_catalog_key_for_label(SELF_LUBIKAN_CAPTURE_LABEL)),
				selected: true,
				rank: None,
				event: None,
			}],
		),
	)
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

#[cfg(test)]
mod tests {
	use super::{InventoryLayout, InventoryLayoutMode, STARTER_KIT_LABEL};
	use crate::MenuEvent;
	use character_items::{
		self_lubikan_catalog_key, ClothingMaterial, ClothingMesh, FirearmMesh, Inventory,
		InventoryItem, ItemColor, ItemRng, SkillMapKind, SkillMapSpec, STARTER_CLOTHING_COUNT,
	};
	use character_ui_menu::MenuNode;

	/// Test-only bag large enough that Clothing + Tools overflow a short pane
	/// even at four 52px-thumb columns. Never used by generation or the real
	/// Starter Kit.
	fn overflow_inventory_items() -> Vec<InventoryItem> {
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
		assert_eq!(
			starter.section_labels(),
			["Clothing", "Tools", "Skill Maps", "Tanzes", "Lubikan"]
		);
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
		// Shared inventory is four 52px-thumb columns. 12 clothing + 5
		// firearms + 1 grenade + 2 skill maps is ~20 tiles → at least five
		// rows, plus four section headers. A 400px pane cannot show that.
		assert!(tiles >= 18, "overflow bag should have many tiles, got {tiles}");
	}

	fn clothing_tile_count(layout: &InventoryLayout) -> usize {
		let MenuNode::Section { children, .. } = &layout.node else {
			return 0;
		};
		for child in children {
			let MenuNode::Section { label: "Clothing", children } = child else {
				continue;
			};
			for inner in children {
				if let MenuNode::GridCatalog { choices, .. } = inner {
					return choices.len();
				}
			}
		}
		0
	}

	#[test]
	fn generated_starter_kit_has_exactly_three_clothing_tiles() {
		let mut rng = ItemRng::from_seed(42);
		let items = rng.random_starter_loadout();
		assert_eq!(
			items.iter().filter(|item| item.mesh().is_some()).count(),
			STARTER_CLOTHING_COUNT
		);
		let layout = InventoryLayout::starter_kit(items);
		assert_eq!(clothing_tile_count(&layout), STARTER_CLOTHING_COUNT);
		assert_eq!(STARTER_CLOTHING_COUNT, 3);
	}

	#[test]
	fn tanz_tiles_include_owned_unequipped_and_use_pose_catalog_keys() {
		use character_items::TanzId;
		use tanz_user::TanzPose;

		let items = vec![
			InventoryItem::tanz(TanzId::HealingSalute),
			InventoryItem::tanz(TanzId::LeftStrafe),
			InventoryItem::firearm(FirearmMesh::Bullpup),
		];
		let mut inventory = Inventory::with_starter_outfit(items);
		assert!(inventory.toggle(
			inventory
				.items
				.iter()
				.position(|item| item.tanz_id() == Some(TanzId::HealingSalute))
				.expect("salute")
		));
		let layout = InventoryLayout::character_menu(&inventory);
		let MenuNode::Section { children, .. } = &layout.node else {
			panic!("inventory must be a section");
		};
		let tanzes = children.iter().find_map(|child| match child {
			MenuNode::Section { label: "Tanzes", children } => children.first(),
			_ => None,
		});
		let Some(MenuNode::GridCatalog { choices, max_selected, .. }) = tanzes else {
			panic!("Tanzes must be a catalog");
		};
		assert_eq!(*max_selected, 4);
		assert_eq!(choices.len(), 2);
		assert_eq!(choices[0].label, "Healing Salute");
		assert_eq!(choices[0].detail, "A · Forward");
		assert_eq!(choices[0].image_key, Some(TanzPose(TanzId::HealingSalute).icon_catalog_key()));
		assert_eq!(choices[0].rank, Some(1));
		assert!(choices[0].selected);
		assert_eq!(choices[1].label, "Left Strafe");
		assert!(!choices[1].selected);
		assert_eq!(choices[1].rank, None);
		assert!(layout.captures.iter().any(|need| {
			need.silhouette && need.key == TanzPose(TanzId::LeftStrafe).icon_catalog_key()
		}));
	}

	#[test]
	fn both_layouts_use_the_self_lubikan_catalog_tile() {
		let starter = InventoryLayout::starter_kit(sample_items());
		let menu = InventoryLayout::character_menu(&Inventory::with_starter_outfit(sample_items()));
		for layout in [&starter, &menu] {
			let MenuNode::Section { children, .. } = &layout.node else {
				panic!("inventory must be a section");
			};
			let lubikan = children.iter().find_map(|child| match child {
				MenuNode::Section { label: "Lubikan", children } => children.first(),
				_ => None,
			});
			let Some(MenuNode::GridCatalog { choices, .. }) = lubikan else {
				panic!("Lubikan must be a catalog tile");
			};
			assert_eq!(choices.len(), 1);
			assert_eq!(choices[0].label, "Self");
			assert_eq!(choices[0].detail, "Always equipped");
			assert_eq!(choices[0].image_key, Some(self_lubikan_catalog_key()));
			assert!(choices[0].selected);
			assert_eq!(choices[0].event, None);
		}
	}

	#[test]
	fn layout_stays_loading_until_every_capture_reports() {
		use super::{inventory_paint_state, InventoryPaint, INVENTORY_CAPTURE_TIMEOUT};
		use std::time::Duration;

		let layout = InventoryLayout::starter_kit(sample_items());
		assert!(!layout.captures.is_empty());
		assert_eq!(
			inventory_paint_state(
				layout.captures.iter().map(|_| false),
				Duration::ZERO,
				INVENTORY_CAPTURE_TIMEOUT
			),
			InventoryPaint::Loading
		);
		let mut some = vec![false; layout.captures.len()];
		some[0] = true;
		assert_eq!(
			inventory_paint_state(some, Duration::ZERO, INVENTORY_CAPTURE_TIMEOUT),
			InventoryPaint::Loading
		);
		assert_eq!(
			inventory_paint_state(
				layout.captures.iter().map(|_| true),
				Duration::ZERO,
				INVENTORY_CAPTURE_TIMEOUT
			),
			InventoryPaint::Ready
		);
	}

	#[test]
	fn timeout_reveals_without_every_report() {
		use super::{inventory_paint_state, InventoryPaint, INVENTORY_CAPTURE_TIMEOUT};

		let layout = InventoryLayout::starter_kit(sample_items());
		assert_eq!(
			inventory_paint_state(
				layout.captures.iter().map(|_| false),
				INVENTORY_CAPTURE_TIMEOUT,
				INVENTORY_CAPTURE_TIMEOUT
			),
			InventoryPaint::Ready
		);
	}
}
