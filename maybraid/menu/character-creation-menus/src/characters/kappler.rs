use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		common::EyeMesh,
		kappler::{
			KapplerBeakColor, KapplerBeakMesh, KapplerColors, KapplerConfig, KapplerEyeColor,
			KapplerHeadMesh, KapplerPlumageColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{
		KAPPLER_EYE_FOCUS, KAPPLER_HEAD_ROOT_FOCUS, KAPPLER_MOUTH_FOCUS, SMALL_BIRD_BODY_FOCUS,
	},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct KapplerHeadMenu {
	pub head: AssetSingleSelect<KapplerHeadMesh>,
	pub plumage: SwatchSingleSelect<KapplerPlumageColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KapplerHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub beak: AssetSingleSelect<KapplerBeakMesh>,
	pub eye_color: SwatchSingleSelect<KapplerEyeColor>,
	pub beak_color: SwatchSingleSelect<KapplerBeakColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KapplerMenu {
	pub head: Section<KapplerHeadMenu>,
	pub head_features: Section<KapplerHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&KapplerConfig> for KapplerMenu {
	fn from(config: &KapplerConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				KapplerHeadMenu {
					head: AssetSingleSelect::new(KapplerHeadMesh::Meerkat)
						.with_camera_focus(KAPPLER_HEAD_ROOT_FOCUS),
					plumage: SwatchSingleSelect::new(config.colors.plumage),
				},
			),
			head_features: Section::new(
				"Head & Features",
				KapplerHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(KAPPLER_EYE_FOCUS),
					beak: AssetSingleSelect::new(config.beak)
						.with_camera_focus(KAPPLER_MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					beak_color: SwatchSingleSelect::new(config.colors.beak),
				},
			),
			hair: Section::new(
				"Hair",
				HairMenu::new(config.hair, config.colors.hair, KAPPLER_HEAD_ROOT_FOCUS),
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

impl From<&KapplerMenu> for KapplerConfig {
	fn from(menu: &KapplerMenu) -> Self {
		Self {
			beak: menu.head_features.value.beak.value,
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: KapplerColors {
				plumage: menu.head.value.plumage.value,
				eyes: menu.head_features.value.eye_color.value,
				beak: menu.head_features.value.beak_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MenuComponent<MenuEvent> for KapplerHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Head",
				&self.head,
				PreviewColor::of(self.plumage.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::KapplerHead, AssetValue::KapplerHead(value))
				},
			),
			MenuNode::swatch("Plumage", &self.plumage, |color| {
				MenuEvent::SetSwatch(
					CharacterField::KapplerPlumageColor,
					SwatchValue::KapplerPlumage(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for KapplerHeadFeaturesMenu {
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
					CharacterField::KapplerEyeColor,
					SwatchValue::KapplerEye(color),
				)
			}),
			MenuNode::asset_grid(
				"Beak",
				&self.beak,
				PreviewColor::of(self.beak_color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::KapplerBeak, AssetValue::KapplerBeak(value))
				},
			),
			MenuNode::swatch("Beak Color", &self.beak_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::KapplerBeakColor,
					SwatchValue::KapplerBeak(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for KapplerMenu {
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

impl KapplerMenu {
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
			CharacterField::KapplerHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::KapplerBeak => self.head_features.value.beak.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(SMALL_BIRD_BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for KapplerMenu {
	fn default() -> Self {
		Self::from(&KapplerConfig::default_preview())
	}
}
