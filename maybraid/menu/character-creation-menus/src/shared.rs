//! Menu sections shared by every species.
//!
//! Hair, clothing, and animation controls are structurally identical across
//! species, so they are defined once here instead of per species. Each
//! implements [`MenuComponent`] to lower itself into [`MenuNode`]s.

use character_items::{
	CharacterSheet, ClothingColor, ClothingMaterial, ClothingMaterialChoice, ClothingMesh,
	Inventory, InventoryItem, ItemColor, WORN_CLOTHING_LIMIT,
};
use character_ui_menu::{
	AssetChoice, AssetOption, AssetSingleSelect, CameraFocus, GridCatalogChoice, ItemRow,
	MenuComponent, MenuNode, MultiSelect, PreviewColor, SelectGroup, SingleSelect, StatCard,
	StatLine, SwatchChoice, SwatchSingleSelect,
};
use characters::ConceptAnimation;

use crate::{
	cycle_value,
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
};

/// Hair style and color; species only differ in default camera framing.
#[derive(Clone, Debug, PartialEq)]
pub struct HairMenu {
	pub style: AssetSingleSelect<characters::species::common::HairMesh>,
	pub color: SwatchSingleSelect<ItemColor>,
}

impl HairMenu {
	pub fn new(
		style: characters::species::common::HairMesh,
		color: ItemColor,
		focus: CameraFocus,
	) -> Self {
		Self {
			style: AssetSingleSelect::new(style).with_camera_focus(focus),
			color: SwatchSingleSelect::new(color),
		}
	}
}

impl MenuComponent<MenuEvent> for HairMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Hair",
				&self.style,
				PreviewColor::of(self.color.value),
				|value| MenuEvent::SetAsset(CharacterField::Hair, AssetValue::Hair(value)),
			),
			MenuNode::swatch("Hair Color", &self.color, |color| {
				MenuEvent::SetSwatch(CharacterField::HairColor, SwatchValue::Item(color))
			}),
		])
	}
}

/// Clothing layers with a default color/look plus per-layer overrides.
#[derive(Clone, Debug, PartialEq)]
pub struct ClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<ItemColor>,
	pub item_colors: Vec<ClothingColor>,
	pub material: SingleSelect<ClothingMaterial>,
	pub item_materials: Vec<ClothingMaterialChoice>,
	/// When set, the clothing section is an owned-item [`MenuNode::GridCatalog`]
	/// instead of the full mesh catalog.
	pub owned: Option<Vec<InventoryItem>>,
}

impl ClothingMenu {
	pub fn new(
		layers: Vec<ClothingMesh>,
		default_color: ItemColor,
		overrides: Vec<ClothingColor>,
		material: ClothingMaterial,
		material_overrides: Vec<ClothingMaterialChoice>,
	) -> Self {
		Self {
			layers: MultiSelect::new(layers),
			default_color: SwatchSingleSelect::new(default_color),
			item_colors: overrides,
			material: SingleSelect::new(material),
			item_materials: material_overrides,
			owned: None,
		}
	}

	pub fn color_for(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.item_colors, self.default_color.value, clothing)
	}

	pub fn set_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.item_colors, clothing, color);
	}

	pub fn material_for(&self, clothing: ClothingMesh) -> ClothingMaterial {
		ClothingMaterialChoice::resolve(&self.item_materials, self.material.value, clothing)
	}

	pub fn set_material(&mut self, clothing: ClothingMesh, material: ClothingMaterial) {
		ClothingMaterialChoice::set(&mut self.item_materials, clothing, material);
	}
}

/// Clothing toggle / color / material events shared by every species menu.
pub(crate) fn apply_clothing_event(menu: &mut ClothingMenu, event: MenuEvent) -> bool {
	match event {
		MenuEvent::ToggleClothing(clothing) => {
			menu.layers.toggle(clothing);
			true
		}
		MenuEvent::SetSwatch(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
			menu.set_color(clothing, color);
			true
		}
		MenuEvent::SetAsset(
			CharacterField::ClothingMaterial(clothing),
			AssetValue::ClothingMaterial(material),
		) => {
			menu.set_material(clothing, material);
			true
		}
		MenuEvent::Cycle(CharacterField::ClothingMaterial(clothing), delta) => {
			menu.set_material(clothing, cycle_value(menu.material_for(clothing), delta));
			true
		}
		_ => false,
	}
}

impl MenuComponent<MenuEvent> for ClothingMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		if let Some(owned) = &self.owned {
			return inventory_catalog(self, owned);
		}
		let rows = ClothingMesh::VALUES
			.iter()
			.map(|&clothing| {
				let selected = self.layers.contains(clothing);
				let color = self.color_for(clothing);
				ItemRow {
					asset: AssetChoice::new(
						clothing,
						selected,
						MenuEvent::ToggleClothing(clothing),
					),
					preview: PreviewColor::of(color),
					// Color and look choices only matter for worn layers.
					colors: if selected {
						SwatchChoice::from_values(color, |color| {
							MenuEvent::SetSwatch(
								CharacterField::Clothing(clothing),
								SwatchValue::Item(color),
							)
						})
					} else {
						Vec::new()
					},
					materials: if selected {
						SelectGroup::from_values(
							None,
							self.material_for(clothing),
							|material| {
								MenuEvent::SetAsset(
									CharacterField::ClothingMaterial(clothing),
									AssetValue::ClothingMaterial(material),
								)
							},
							ClothingMaterial::VALUES,
						)
						.choices
					} else {
						Vec::new()
					},
				}
			})
			.collect();
		MenuNode::ItemMultiSelect { label: "Clothing", rows }
	}
}

fn inventory_catalog(menu: &ClothingMenu, owned: &[InventoryItem]) -> MenuNode<MenuEvent> {
	MenuNode::grid_catalog(
		"Clothing",
		WORN_CLOTHING_LIMIT,
		owned.iter().enumerate().filter_map(|(index, item)| {
			let mesh = item.mesh()?;
			let asset = mesh.asset();
			Some(GridCatalogChoice {
				label: item.name(),
				detail: item.catalog_detail(),
				path: asset.path,
				thumbnail_camera: asset.thumbnail_camera,
				preview: PreviewColor::of(item.material()?.color),
				image_key: item.icon_catalog_key(),
				selected: menu.layers.contains(mesh),
				rank: menu
					.layers
					.selected
					.iter()
					.position(|worn| *worn == mesh)
					.map(|position| (position + 1) as u8),
				event: MenuEvent::ToggleInventory(index),
			})
		}),
	)
}

pub(crate) fn clothing_menu_from_inventory(inventory: &Inventory) -> ClothingMenu {
	let worn: Vec<ClothingMesh> = inventory.worn_items().filter_map(InventoryItem::mesh).collect();
	let item_colors: Vec<ClothingColor> = inventory
		.worn_items()
		.filter_map(|item| {
			item.mesh()
				.zip(item.material())
				.map(|(clothing, material)| ClothingColor { clothing, color: material.color })
		})
		.collect();
	let item_materials: Vec<ClothingMaterialChoice> =
		inventory
			.worn_items()
			.filter_map(|item| {
				item.mesh().zip(item.material()).map(|(clothing, material)| {
					ClothingMaterialChoice { clothing, material: material.id }
				})
			})
			.collect();
	let default_color = inventory
		.items
		.iter()
		.find_map(|item| item.material().map(|material| material.color))
		.unwrap_or_default();
	let default_material = inventory
		.items
		.iter()
		.find_map(|item| item.material().map(|material| material.id))
		.unwrap_or_default();
	let mut menu =
		ClothingMenu::new(worn, default_color, item_colors, default_material, item_materials);
	menu.owned = Some(inventory.items.clone());
	menu
}

/// Overlay / header keys that stay editable after appearance lock.
pub(crate) fn inventory_overlay_key(key: &str) -> bool {
	matches!(
		key,
		"Inventory"
			| "Starter Kit"
			| "Clothing"
			| "Weapons"
			| "Tools" | "Skill Maps"
			| "Lubikan"
			| "Loadout"
	)
}

pub(crate) fn loadout_section(inventory: &Inventory) -> MenuNode<MenuEvent> {
	let total = inventory.character_sheet();
	let buffs = CharacterSheet::modifiers_from_inventory(inventory);
	let mut cards = vec![
		StatCard {
			title: String::from("Total Stats"),
			rows: total
				.stat_rows()
				.into_iter()
				.map(|(label, value)| {
					if label == "Pace" {
						StatLine::formula(label, value)
					} else {
						StatLine::from_display(label, value)
					}
				})
				.collect(),
		},
		StatCard {
			title: String::from("Base Stats"),
			rows: CharacterSheet::base_stat_rows()
				.into_iter()
				.map(|(label, value)| StatLine::unsigned(label, value))
				.collect(),
		},
		StatCard {
			title: String::from("Buffs"),
			rows: buffs
				.buff_stat_rows()
				.into_iter()
				.map(|(label, value)| StatLine::from_display(label, value))
				.collect(),
		},
	];
	for &index in &inventory.weapons {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		cards.push(StatCard {
			title: item.name(),
			rows: item
				.stat_rows()
				.into_iter()
				.map(|(label, value)| StatLine::unsigned(label, value))
				.collect(),
		});
	}
	for &index in &inventory.skills {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		cards.push(StatCard {
			title: item.name(),
			rows: item
				.stat_rows()
				.into_iter()
				.map(|(label, value)| StatLine::from_display(label, value))
				.collect(),
		});
	}
	MenuNode::section("Loadout", MenuNode::stat_grid(cards))
}

/// Animation clip picker.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationMenu {
	pub clip: AssetSingleSelect<ConceptAnimation>,
}

impl AnimationMenu {
	pub fn new(focus: CameraFocus) -> Self {
		Self { clip: AssetSingleSelect::new(ConceptAnimation::Still).with_camera_focus(focus) }
	}
}

impl MenuComponent<MenuEvent> for AnimationMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::asset_grid("Animation", &self.clip, PreviewColor::WHITE, |value| {
			MenuEvent::SetAsset(CharacterField::Animation, AssetValue::Animation(value))
		})
	}
}
