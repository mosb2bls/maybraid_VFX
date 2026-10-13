use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		common::EyeMesh,
		spibmom::{
			SpibmomColors, SpibmomConfig, SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor,
			SpibmomHeadMesh, SpibmomMouthColor, SpibmomMouthMesh, SpibmomSkinColor,
			SpibmomSpineColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{
		SPIBMOM_BODY_FOCUS, SPIBMOM_CROWN_FOCUS, SPIBMOM_EYE_FOCUS, SPIBMOM_HEAD_ROOT_FOCUS,
		SPIBMOM_NOSE_FOCUS,
	},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomHeadMenu {
	pub head: AssetSingleSelect<SpibmomHeadMesh>,
	pub skin: SwatchSingleSelect<SpibmomSkinColor>,
	pub crown_color: SwatchSingleSelect<SpibmomCrownColor>,
	pub spine_color: SwatchSingleSelect<SpibmomSpineColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub snout: AssetSingleSelect<SpibmomMouthMesh>,
	pub eye_color: SwatchSingleSelect<SpibmomEyeColor>,
	pub ear_color: SwatchSingleSelect<SpibmomEarColor>,
	pub mouth_color: SwatchSingleSelect<SpibmomMouthColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomMenu {
	pub head: Section<SpibmomHeadMenu>,
	pub head_features: Section<SpibmomHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&SpibmomConfig> for SpibmomMenu {
	fn from(config: &SpibmomConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				SpibmomHeadMenu {
					head: AssetSingleSelect::new(SpibmomHeadMesh::Meerkat)
						.with_camera_focus(SPIBMOM_HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
					crown_color: SwatchSingleSelect::new(config.colors.crown)
						.with_camera_focus(SPIBMOM_CROWN_FOCUS),
					spine_color: SwatchSingleSelect::new(config.colors.spine),
				},
			),
			head_features: Section::new(
				"Head & Features",
				SpibmomHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(SPIBMOM_EYE_FOCUS),
					snout: AssetSingleSelect::new(SpibmomMouthMesh::Trunkish)
						.with_camera_focus(SPIBMOM_NOSE_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					ear_color: SwatchSingleSelect::new(config.colors.ears),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				HairMenu::new(config.hair, config.colors.hair, SPIBMOM_HEAD_ROOT_FOCUS),
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
			.with_camera_focus(SPIBMOM_BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(SPIBMOM_BODY_FOCUS)),
		}
	}
}

impl From<&SpibmomMenu> for SpibmomConfig {
	fn from(menu: &SpibmomMenu) -> Self {
		Self {
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: SpibmomColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				ears: menu.head_features.value.ear_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				crown: menu.head.value.crown_color.value,
				spine: menu.head.value.spine_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MenuComponent<MenuEvent> for SpibmomHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, PreviewColor::of(self.skin.value), |value| {
				MenuEvent::SetAsset(CharacterField::SpibmomHead, AssetValue::SpibmomHead(value))
			}),
			MenuNode::swatch("Skin", &self.skin, |color| {
				MenuEvent::SetSwatch(
					CharacterField::SpibmomSkinColor,
					SwatchValue::SpibmomSkin(color),
				)
			}),
			MenuNode::swatch("Crown Color", &self.crown_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::SpibmomCrownColor,
					SwatchValue::SpibmomCrown(color),
				)
			}),
			MenuNode::swatch("Spine Color", &self.spine_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::SpibmomSpineColor,
					SwatchValue::SpibmomSpine(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for SpibmomHeadFeaturesMenu {
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
					CharacterField::SpibmomEyeColor,
					SwatchValue::SpibmomEye(color),
				)
			}),
			MenuNode::swatch("Ear Color", &self.ear_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::SpibmomEarColor,
					SwatchValue::SpibmomEar(color),
				)
			}),
			MenuNode::asset_grid(
				"Snout",
				&self.snout,
				PreviewColor::of(self.mouth_color.value),
				|value| {
					MenuEvent::SetAsset(
						CharacterField::SpibmomMouth,
						AssetValue::SpibmomMouth(value),
					)
				},
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::SpibmomMouthColor,
					SwatchValue::SpibmomMouthColor(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for SpibmomMenu {
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

impl SpibmomMenu {
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
			CharacterField::SpibmomHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::SpibmomMouth => self.head_features.value.snout.camera_focus,
			CharacterField::SpibmomCrownColor => self.head.value.crown_color.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation
			| CharacterField::SpibmomSpineColor => Some(SPIBMOM_BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for SpibmomMenu {
	fn default() -> Self {
		Self::from(&SpibmomConfig::default_preview())
	}
}
