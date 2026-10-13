use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		brokker::{
			BrokkerColors, BrokkerConfig, BrokkerEyeColor, BrokkerHeadMesh, BrokkerPlumageColor,
			BrokkerSnoutColor,
		},
		common::EyeMesh,
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct BrokkerHeadMenu {
	pub head: AssetSingleSelect<BrokkerHeadMesh>,
	pub plumage: SwatchSingleSelect<BrokkerPlumageColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrokkerHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub eye_color: SwatchSingleSelect<BrokkerEyeColor>,
	pub snout_color: SwatchSingleSelect<BrokkerSnoutColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrokkerMenu {
	pub head: Section<BrokkerHeadMenu>,
	pub head_features: Section<BrokkerHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&BrokkerConfig> for BrokkerMenu {
	fn from(config: &BrokkerConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				BrokkerHeadMenu {
					head: AssetSingleSelect::new(BrokkerHeadMesh::OrthoTee)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					plumage: SwatchSingleSelect::new(config.colors.plumage),
				},
			),
			head_features: Section::new(
				"Head & Features",
				BrokkerHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					snout_color: SwatchSingleSelect::new(config.colors.snout),
				},
			),
			hair: Section::new(
				"Hair",
				HairMenu::new(config.hair, config.colors.hair, HEAD_ROOT_FOCUS),
			),
			clothing: Section::new(
				"Clothing",
				ClothingMenu::new(
					config.clothing.clone(),
					config.colors.clothing_default,
					config.colors.clothing.clone(),
					config.colors.clothing_material,
					config.colors.clothing_materials.clone(),
				),
			)
			.with_camera_focus(BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(BODY_FOCUS)),
		}
	}
}

impl From<&BrokkerMenu> for BrokkerConfig {
	fn from(menu: &BrokkerMenu) -> Self {
		Self {
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: BrokkerColors {
				plumage: menu.head.value.plumage.value,
				eyes: menu.head_features.value.eye_color.value,
				snout: menu.head_features.value.snout_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MenuComponent<MenuEvent> for BrokkerHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Head",
				&self.head,
				PreviewColor::of(self.plumage.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::BrokkerHead, AssetValue::BrokkerHead(value))
				},
			),
			MenuNode::swatch("Plumage", &self.plumage, |color| {
				MenuEvent::SetSwatch(
					CharacterField::BrokkerPlumageColor,
					SwatchValue::BrokkerPlumage(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BrokkerHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::BrokkerEyeColor,
					SwatchValue::BrokkerEye(color),
				)
			}),
			MenuNode::swatch("Snout Color", &self.snout_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::BrokkerSnoutColor,
					SwatchValue::BrokkerSnout(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BrokkerMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.head.label, self.head.value.menu_node()),
			MenuNode::section(self.head_features.label, self.head_features.value.menu_node()),
			MenuNode::section(self.hair.label, self.hair.value.menu_node()),
			MenuNode::section(self.clothing.label, self.clothing.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl BrokkerMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = animation;
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value
	}

	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		self.clothing.value.color_for(clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		self.clothing.value.set_color(clothing, color);
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::BrokkerHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for BrokkerMenu {
	fn default() -> Self {
		Self::from(&BrokkerConfig::default_preview())
	}
}
