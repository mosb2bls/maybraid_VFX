use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		brodler::{
			assets::HornMesh, BrodlerColors, BrodlerConfig, BrodlerEyeColor, BrodlerHeadMesh,
			BrodlerHornColor, BrodlerSkinColor,
		},
		common::{EarMesh, EyeMesh, MouthMesh, NoseMesh},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{
		BODY_FOCUS, CROWN_FOCUS, EAR_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS, NOSE_FOCUS,
	},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerHeadMenu {
	pub head: AssetSingleSelect<BrodlerHeadMesh>,
	pub horns: AssetSingleSelect<HornMesh>,
	pub skin: SwatchSingleSelect<BrodlerSkinColor>,
	/// Mirror of the horn color swatch (owned by the features section);
	/// tints the horn previews. Kept in sync when that swatch changes.
	pub horn_color: BrodlerHornColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub nose: AssetSingleSelect<NoseMesh>,
	pub mouth: AssetSingleSelect<MouthMesh>,
	pub ear: AssetSingleSelect<EarMesh>,
	pub eye_color: SwatchSingleSelect<BrodlerEyeColor>,
	pub horn_color: SwatchSingleSelect<BrodlerHornColor>,
	pub mouth_color: SwatchSingleSelect<ItemColor>,
	/// Mirror of the skin color swatch (owned by the head section); tints the
	/// meshes that inherit it (nose, ears). Kept in sync when it changes.
	pub skin_color: BrodlerSkinColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerMenu {
	pub head: Section<BrodlerHeadMenu>,
	pub head_features: Section<BrodlerHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&BrodlerConfig> for BrodlerMenu {
	fn from(config: &BrodlerConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				BrodlerHeadMenu {
					head: AssetSingleSelect::new(config.head).with_camera_focus(HEAD_ROOT_FOCUS),
					horns: AssetSingleSelect::new(config.horns).with_camera_focus(CROWN_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
					horn_color: config.colors.horns,
				},
			),
			head_features: Section::new(
				"Head & Features",
				BrodlerHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					nose: AssetSingleSelect::new(config.nose).with_camera_focus(NOSE_FOCUS),
					mouth: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					ear: AssetSingleSelect::new(config.ear).with_camera_focus(EAR_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					horn_color: SwatchSingleSelect::new(config.colors.horns),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
					skin_color: config.colors.skin,
				},
			),
			hair: Section::new("Hair", HairMenu::new(config.hair, config.colors.hair, CROWN_FOCUS)),
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

impl From<&BrodlerMenu> for BrodlerConfig {
	fn from(menu: &BrodlerMenu) -> Self {
		Self {
			head: menu.head.value.head.value,
			horns: menu.head.value.horns.value,
			eye: menu.head_features.value.eye.value,
			nose: menu.head_features.value.nose.value,
			mouth: menu.head_features.value.mouth.value,
			ear: menu.head_features.value.ear.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: BrodlerColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				horns: menu.head_features.value.horn_color.value,
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

impl MenuComponent<MenuEvent> for BrodlerHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		let base = PreviewColor::of(self.skin.value);
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, base, |value| {
				MenuEvent::SetAsset(CharacterField::BrodlerHead, AssetValue::BrodlerHead(value))
			}),
			MenuNode::asset_grid(
				"Horns",
				&self.horns,
				PreviewColor::of(self.horn_color),
				|value| MenuEvent::SetAsset(CharacterField::Horns, AssetValue::Horns(value)),
			),
			MenuNode::swatch("Skin", &self.skin, |color| {
				MenuEvent::SetSwatch(CharacterField::SkinColor, SwatchValue::BrodlerSkin(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BrodlerHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		let base = PreviewColor::of(self.skin_color);
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::BrodlerEyeColor,
					SwatchValue::BrodlerEye(color),
				)
			}),
			MenuNode::swatch("Horn Color", &self.horn_color, |color| {
				MenuEvent::SetSwatch(CharacterField::HornColor, SwatchValue::BrodlerHorn(color))
			}),
			MenuNode::asset_grid("Nose", &self.nose, base, |value| {
				MenuEvent::SetAsset(CharacterField::Nose, AssetValue::Nose(value))
			}),
			MenuNode::asset_grid(
				"Mouth",
				&self.mouth,
				PreviewColor::of(self.mouth_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Mouth, AssetValue::Mouth(value)),
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(CharacterField::MouthColor, SwatchValue::Item(color))
			}),
			MenuNode::asset_grid("Ears", &self.ear, base, |value| {
				MenuEvent::SetAsset(CharacterField::Ear, AssetValue::Ear(value))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BrodlerMenu {
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

impl BrodlerMenu {
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
			CharacterField::BrodlerHead => self.head.value.head.camera_focus,
			CharacterField::Horns => self.head.value.horns.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::Nose => self.head_features.value.nose.camera_focus,
			CharacterField::Mouth => self.head_features.value.mouth.camera_focus,
			CharacterField::Ear => self.head_features.value.ear.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for BrodlerMenu {
	fn default() -> Self {
		Self::from(&BrodlerConfig::default_preview())
	}
}
