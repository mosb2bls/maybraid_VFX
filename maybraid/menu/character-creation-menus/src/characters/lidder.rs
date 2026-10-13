use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		common::EyeMesh,
		lidder::{
			LidderBeakColor, LidderBeakMesh, LidderColors, LidderConfig, LidderEyeColor,
			LidderHeadMesh, LidderPlumageColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct LidderHeadMenu {
	pub head: AssetSingleSelect<LidderHeadMesh>,
	pub plumage: SwatchSingleSelect<LidderPlumageColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LidderHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub beak: AssetSingleSelect<LidderBeakMesh>,
	pub eye_color: SwatchSingleSelect<LidderEyeColor>,
	pub beak_color: SwatchSingleSelect<LidderBeakColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LidderMenu {
	pub head: Section<LidderHeadMenu>,
	pub head_features: Section<LidderHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&LidderConfig> for LidderMenu {
	fn from(config: &LidderConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				LidderHeadMenu {
					head: AssetSingleSelect::new(LidderHeadMesh::Meerkat)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					plumage: SwatchSingleSelect::new(config.colors.plumage),
				},
			),
			head_features: Section::new(
				"Head & Features",
				LidderHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					beak: AssetSingleSelect::new(config.beak).with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					beak_color: SwatchSingleSelect::new(config.colors.beak),
				},
			),
			hair: Section::new(
				"Hair",
				// Crest tint follows plumage; HairMenu still stores a color for round-trip.
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

impl From<&LidderMenu> for LidderConfig {
	fn from(menu: &LidderMenu) -> Self {
		Self {
			beak: menu.head_features.value.beak.value,
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: LidderColors {
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

impl MenuComponent<MenuEvent> for LidderHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Head",
				&self.head,
				PreviewColor::of(self.plumage.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::LidderHead, AssetValue::LidderHead(value))
				},
			),
			MenuNode::swatch("Plumage", &self.plumage, |color| {
				MenuEvent::SetSwatch(
					CharacterField::LidderPlumageColor,
					SwatchValue::LidderPlumage(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for LidderHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::LidderEyeColor, SwatchValue::LidderEye(color))
			}),
			MenuNode::asset_grid(
				"Beak",
				&self.beak,
				PreviewColor::of(self.beak_color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::LidderBeak, AssetValue::LidderBeak(value))
				},
			),
			MenuNode::swatch("Beak Color", &self.beak_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::LidderBeakColor,
					SwatchValue::LidderBeak(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for LidderMenu {
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

impl LidderMenu {
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
			CharacterField::LidderHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::LidderBeak => self.head_features.value.beak.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for LidderMenu {
	fn default() -> Self {
		Self::from(&LidderConfig::default_preview())
	}
}
