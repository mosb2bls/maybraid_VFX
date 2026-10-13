use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section, SingleSelect,
	SwatchSingleSelect,
};
use characters::{
	species::{
		common::EyeMesh,
		wumbus::{
			WumbusColors, WumbusConfig, WumbusEarColor, WumbusEyeColor, WumbusHeadMesh,
			WumbusHornColor, WumbusHornMesh, WumbusMouthColor, WumbusMouthMesh, WumbusSkinColor,
			WumbusSpineColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{BODY_FOCUS, CROWN_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusHeadMenu {
	pub head: AssetSingleSelect<WumbusHeadMesh>,
	pub skin: SwatchSingleSelect<WumbusSkinColor>,
	pub horns: SingleSelect<WumbusHornMesh>,
	pub horn_color: SwatchSingleSelect<WumbusHornColor>,
	pub spine_color: SwatchSingleSelect<WumbusSpineColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub snout: AssetSingleSelect<WumbusMouthMesh>,
	pub eye_color: SwatchSingleSelect<WumbusEyeColor>,
	pub ear_color: SwatchSingleSelect<WumbusEarColor>,
	pub mouth_color: SwatchSingleSelect<WumbusMouthColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusMenu {
	pub head: Section<WumbusHeadMenu>,
	pub head_features: Section<WumbusHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&WumbusConfig> for WumbusMenu {
	fn from(config: &WumbusConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				WumbusHeadMenu {
					head: AssetSingleSelect::new(WumbusHeadMesh::OrthoBear)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
					horns: SingleSelect::new(config.horns).with_camera_focus(CROWN_FOCUS),
					horn_color: SwatchSingleSelect::new(config.colors.horns),
					spine_color: SwatchSingleSelect::new(config.colors.spine),
				},
			),
			head_features: Section::new(
				"Head & Features",
				WumbusHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					snout: AssetSingleSelect::new(WumbusMouthMesh::CanineSnout)
						.with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					ear_color: SwatchSingleSelect::new(config.colors.ears),
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

impl From<&WumbusMenu> for WumbusConfig {
	fn from(menu: &WumbusMenu) -> Self {
		Self {
			horns: menu.head.value.horns.value,
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: WumbusColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				ears: menu.head_features.value.ear_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				horns: menu.head.value.horn_color.value,
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

impl MenuComponent<MenuEvent> for WumbusHeadMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, PreviewColor::of(self.skin.value), |value| {
				MenuEvent::SetAsset(CharacterField::WumbusHead, AssetValue::WumbusHead(value))
			}),
			MenuNode::swatch("Fur", &self.skin, |color| {
				MenuEvent::SetSwatch(
					CharacterField::WumbusSkinColor,
					SwatchValue::WumbusSkin(color),
				)
			}),
			MenuNode::cycle("Horns", &self.horns, |delta| {
				MenuEvent::Cycle(CharacterField::WumbusHorns, delta)
			}),
			MenuNode::swatch("Horn Color", &self.horn_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::WumbusHornColor,
					SwatchValue::WumbusHorn(color),
				)
			}),
			MenuNode::swatch("Spine Color", &self.spine_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::WumbusSpineColor,
					SwatchValue::WumbusSpine(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for WumbusHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::WumbusEyeColor, SwatchValue::WumbusEye(color))
			}),
			MenuNode::swatch("Ear Color", &self.ear_color, |color| {
				MenuEvent::SetSwatch(CharacterField::WumbusEarColor, SwatchValue::WumbusEar(color))
			}),
			MenuNode::asset_grid(
				"Snout",
				&self.snout,
				PreviewColor::of(self.mouth_color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::WumbusMouth, AssetValue::WumbusMouth(value))
				},
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::WumbusMouthColor,
					SwatchValue::WumbusMouth(color),
				)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for WumbusMenu {
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

impl WumbusMenu {
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
			CharacterField::WumbusHead => self.head.value.head.camera_focus,
			CharacterField::WumbusHorns => {
				if self.head.value.horns.value == WumbusHornMesh::None {
					self.head.value.head.camera_focus
				} else {
					self.head.value.horns.camera_focus
				}
			}
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::WumbusMouth => self.head_features.value.snout.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for WumbusMenu {
	fn default() -> Self {
		Self::from(&WumbusConfig::default_preview())
	}
}
