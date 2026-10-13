use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section,
	SwatchSingleSelect,
};
use characters::{
	species::{
		common::EyeMesh,
		mygr::{MygrColors, MygrConfig, MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct MygrHeadMenu {
	pub head: AssetSingleSelect<MygrHeadMesh>,
	pub skin: SwatchSingleSelect<MygrSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MygrHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub snout: AssetSingleSelect<MygrMouthMesh>,
	pub eye_color: SwatchSingleSelect<MygrEyeColor>,
	pub mouth_color: SwatchSingleSelect<ItemColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MygrMenu {
	pub head: Section<MygrHeadMenu>,
	pub head_features: Section<MygrHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&MygrConfig> for MygrMenu {
	fn from(config: &MygrConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				MygrHeadMenu {
					head: AssetSingleSelect::new(MygrHeadMesh::OrthoBear)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
				},
			),
			head_features: Section::new(
				"Head & Features",
				MygrHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					snout: AssetSingleSelect::new(MygrMouthMesh::CanineSnout)
						.with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
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

impl From<&MygrMenu> for MygrConfig {
	fn from(menu: &MygrMenu) -> Self {
		Self {
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: MygrColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
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

impl MenuComponent<MenuEvent> for MygrHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, PreviewColor::of(self.skin.value), |value| {
				MenuEvent::SetAsset(CharacterField::MygrHead, AssetValue::MygrHead(value))
			}),
			MenuNode::swatch("Fur", &self.skin, |color| {
				MenuEvent::SetSwatch(CharacterField::MygrSkinColor, SwatchValue::MygrSkin(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for MygrHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::MygrEyeColor, SwatchValue::MygrEye(color))
			}),
			MenuNode::asset_grid(
				"Snout",
				&self.snout,
				PreviewColor::of(self.mouth_color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::MygrMouth, AssetValue::MygrMouth(value))
				},
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(CharacterField::MouthColor, SwatchValue::Item(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for MygrMenu {
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

impl MygrMenu {
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
			CharacterField::MygrHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::MygrMouth => self.head_features.value.snout.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for MygrMenu {
	fn default() -> Self {
		Self::from(&MygrConfig::default_preview())
	}
}
