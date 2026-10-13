use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section, SingleSelect,
	SwatchSingleSelect,
};
use characters::{
	species::dui::{
		DuiColors, DuiConfig, DuiEyeColor, DuiEyeMesh, DuiHeadMesh, DuiMouthColor, DuiMouthMesh,
		DuiNoseColor, DuiNoseMesh, DuiSkinColor,
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS, NOSE_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct DuiHeadMenu {
	pub head: AssetSingleSelect<DuiHeadMesh>,
	pub skin: SwatchSingleSelect<DuiSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuiHeadFeaturesMenu {
	pub eye: AssetSingleSelect<DuiEyeMesh>,
	pub nose: SingleSelect<DuiNoseMesh>,
	pub mouth: AssetSingleSelect<DuiMouthMesh>,
	pub mouth_color: SwatchSingleSelect<DuiMouthColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuiMenu {
	pub head: Section<DuiHeadMenu>,
	pub head_features: Section<DuiHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&DuiConfig> for DuiMenu {
	fn from(config: &DuiConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				DuiHeadMenu {
					head: AssetSingleSelect::new(DuiHeadMesh::BarredBowl)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
				},
			),
			head_features: Section::new(
				"Head & Features",
				DuiHeadFeaturesMenu {
					eye: AssetSingleSelect::new(DuiEyeMesh::Thorn).with_camera_focus(EYE_FOCUS),
					nose: SingleSelect::new(config.nose),
					mouth: AssetSingleSelect::new(DuiMouthMesh::SmallCommon)
						.with_camera_focus(MOUTH_FOCUS),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
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

impl From<&DuiMenu> for DuiConfig {
	fn from(menu: &DuiMenu) -> Self {
		Self {
			nose: menu.head_features.value.nose.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: DuiColors {
				skin: menu.head.value.skin.value,
				eyes: DuiEyeColor::Black,
				nose_color: DuiNoseColor::Black,
				mouth: menu.head_features.value.mouth_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MenuComponent<MenuEvent> for DuiHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, PreviewColor::of(self.skin.value), |value| {
				MenuEvent::SetAsset(CharacterField::DuiHead, AssetValue::DuiHead(value))
			}),
			MenuNode::swatch("Skin", &self.skin, |color| {
				MenuEvent::SetSwatch(CharacterField::DuiSkinColor, SwatchValue::DuiSkin(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for DuiHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(DuiEyeColor::Black),
				|value| MenuEvent::SetAsset(CharacterField::DuiEye, AssetValue::DuiEye(value)),
			),
			MenuNode::cycle("Nose", &self.nose, |delta| {
				MenuEvent::Cycle(CharacterField::DuiNose, delta)
			}),
			MenuNode::asset_grid(
				"Mouth",
				&self.mouth,
				PreviewColor::of(self.mouth_color.value),
				|value| MenuEvent::SetAsset(CharacterField::DuiMouth, AssetValue::DuiMouth(value)),
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(CharacterField::DuiMouthColor, SwatchValue::DuiMouth(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for DuiMenu {
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

impl DuiMenu {
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
			CharacterField::DuiHead => self.head.value.head.camera_focus,
			CharacterField::DuiEye => self.head_features.value.eye.camera_focus,
			CharacterField::DuiNose => {
				if self.head_features.value.nose.value == DuiNoseMesh::None {
					None
				} else {
					Some(NOSE_FOCUS)
				}
			}
			CharacterField::DuiMouth => self.head_features.value.mouth.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for DuiMenu {
	fn default() -> Self {
		Self::from(&DuiConfig::default_preview())
	}
}
