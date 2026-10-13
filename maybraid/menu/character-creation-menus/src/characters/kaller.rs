use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		common::EyeMesh,
		kaller::{
			KallerColors, KallerConfig, KallerCrownColor, KallerEyeColor, KallerHeadMesh,
			KallerPlumageColor, KallerSnoutColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{SMALL_BIRD_BODY_FOCUS, SMALL_BIRD_EYE_FOCUS, SMALL_BIRD_HEAD_ROOT_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct KallerHeadMenu {
	pub head: AssetSingleSelect<KallerHeadMesh>,
	pub plumage: SwatchSingleSelect<KallerPlumageColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KallerHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub eye_color: SwatchSingleSelect<KallerEyeColor>,
	pub snout_color: SwatchSingleSelect<KallerSnoutColor>,
	pub crown_color: SwatchSingleSelect<KallerCrownColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KallerMenu {
	pub head: Section<KallerHeadMenu>,
	pub head_features: Section<KallerHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&KallerConfig> for KallerMenu {
	fn from(config: &KallerConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				KallerHeadMenu {
					head: AssetSingleSelect::new(KallerHeadMesh::Meerkat)
						.with_camera_focus(SMALL_BIRD_HEAD_ROOT_FOCUS),
					plumage: SwatchSingleSelect::new(config.colors.plumage),
				},
			),
			head_features: Section::new(
				"Head & Features",
				KallerHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(SMALL_BIRD_EYE_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					snout_color: SwatchSingleSelect::new(config.colors.snout),
					crown_color: SwatchSingleSelect::new(config.colors.crown),
				},
			),
			hair: Section::new(
				"Hair",
				HairMenu::new(config.hair, config.colors.hair, SMALL_BIRD_HEAD_ROOT_FOCUS),
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
			.with_camera_focus(SMALL_BIRD_BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(SMALL_BIRD_BODY_FOCUS)),
		}
	}
}

impl From<&KallerMenu> for KallerConfig {
	fn from(menu: &KallerMenu) -> Self {
		Self {
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: KallerColors {
				plumage: menu.head.value.plumage.value,
				eyes: menu.head_features.value.eye_color.value,
				snout: menu.head_features.value.snout_color.value,
				crown: menu.head_features.value.crown_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MenuComponent<MenuEvent> for KallerHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Head",
				&self.head,
				PreviewColor::of(self.plumage.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::KallerHead, AssetValue::KallerHead(value))
				},
			),
			MenuNode::swatch("Plumage", &self.plumage, |color| {
				MenuEvent::SetSwatch(
					CharacterField::KallerPlumageColor,
					SwatchValue::KallerPlumage(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for KallerHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::KallerEyeColor, SwatchValue::KallerEye(color))
			}),
			MenuNode::swatch("Snout Color", &self.snout_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::KallerSnoutColor,
					SwatchValue::KallerSnout(color),
				)
			}),
			MenuNode::swatch("Crown Color", &self.crown_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::KallerCrownColor,
					SwatchValue::KallerCrown(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for KallerMenu {
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

impl KallerMenu {
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
			CharacterField::KallerHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(SMALL_BIRD_BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for KallerMenu {
	fn default() -> Self {
		Self::from(&KallerConfig::default_preview())
	}
}
