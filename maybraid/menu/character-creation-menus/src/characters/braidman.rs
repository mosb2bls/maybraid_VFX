use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuComponent, MenuNode, PreviewColor, Section, SingleSelect,
	Slider, SwatchSingleSelect,
};
use characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::{sliders::BraidmanSliders, BraidmanColors, BraidmanConfig},
		common::{BodyMesh, EarMesh, EyeMesh, HeadMesh, MouthMesh, NoseMesh},
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
pub struct BraidmanPresetsMenu {
	pub gender: SingleSelect<GenderPreset>,
	pub build: SingleSelect<BuildPreset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanBodyProportionSliders {
	pub shoulder_width: Slider,
	pub hip_width: Slider,
	pub chest_thickness: Slider,
	pub hip_thickness: Slider,
	pub leg_thickness: Slider,
	pub buttocks_thickness: Slider,
	pub waist_thickness: Slider,
	pub lower_trunk_thickness: Slider,
	pub arm_length: Slider,
	pub arm_thickness: Slider,
	pub leg_length: Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanHeadFeatureSliders {
	pub eye_width: Slider,
	pub eye_height: Slider,
	pub eye_tilt: Slider,
	pub nose_width: Slider,
	pub nose_height: Slider,
	pub mouth_width: Slider,
	pub mouth_height: Slider,
	pub ear_width: Slider,
	pub ear_height: Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanBodyMenu {
	pub body: AssetSingleSelect<BodyMesh>,
	pub sliders: BraidmanBodyProportionSliders,
	pub color: SwatchSingleSelect<ItemColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanHeadFeaturesMenu {
	pub head: AssetSingleSelect<HeadMesh>,
	pub eye: AssetSingleSelect<EyeMesh>,
	pub nose: AssetSingleSelect<NoseMesh>,
	pub mouth: AssetSingleSelect<MouthMesh>,
	pub ear: AssetSingleSelect<EarMesh>,
	pub eye_color: SwatchSingleSelect<ItemColor>,
	pub mouth_color: SwatchSingleSelect<ItemColor>,
	pub feature_sliders: BraidmanHeadFeatureSliders,
	/// Mirror of the body color; tints the meshes that inherit it
	/// (head, nose, ears). Kept in sync when the body swatch changes.
	pub body_color: ItemColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanMenu {
	pub presets: Section<BraidmanPresetsMenu>,
	pub body: Section<BraidmanBodyMenu>,
	pub head_features: Section<BraidmanHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

fn slider(value: f32, min: f32, max: f32, step: f32) -> Slider {
	Slider::new(value, min, max, step)
}

impl BraidmanBodyProportionSliders {
	pub fn from_config(sliders: BraidmanSliders) -> Self {
		Self {
			shoulder_width: slider(sliders.shoulder_width, 0.8, 1.2, 0.05),
			hip_width: slider(sliders.hip_width, 0.8, 1.4, 0.05),
			chest_thickness: slider(sliders.chest_thickness, 0.8, 1.2, 0.05),
			hip_thickness: slider(sliders.hip_thickness, 0.8, 1.2, 0.05),
			leg_thickness: slider(sliders.leg_thickness, 0.8, 1.2, 0.05),
			buttocks_thickness: slider(sliders.buttocks_thickness, 0.8, 1.2, 0.05),
			waist_thickness: slider(sliders.waist_thickness, 0.8, 1.2, 0.05),
			lower_trunk_thickness: slider(sliders.lower_trunk_thickness, 0.8, 1.2, 0.05),
			arm_length: slider(sliders.arm_length, 0.8, 1.2, 0.05),
			arm_thickness: slider(sliders.arm_thickness, 0.8, 1.2, 0.05),
			leg_length: slider(sliders.leg_length, 0.8, 1.2, 0.05),
		}
	}

	pub fn write_config(&self, sliders: &mut BraidmanSliders) {
		sliders.shoulder_width = self.shoulder_width.value;
		sliders.hip_width = self.hip_width.value;
		sliders.chest_thickness = self.chest_thickness.value;
		sliders.hip_thickness = self.hip_thickness.value;
		sliders.leg_thickness = self.leg_thickness.value;
		sliders.buttocks_thickness = self.buttocks_thickness.value;
		sliders.waist_thickness = self.waist_thickness.value;
		sliders.lower_trunk_thickness = self.lower_trunk_thickness.value;
		sliders.arm_length = self.arm_length.value;
		sliders.arm_thickness = self.arm_thickness.value;
		sliders.leg_length = self.leg_length.value;
	}
}

impl MenuComponent<MenuEvent> for BraidmanBodyProportionSliders {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::slider("Shoulder Width", &self.shoulder_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::ShoulderWidth, delta)
			}),
			MenuNode::slider("Hip Width", &self.hip_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::HipWidth, delta)
			}),
			MenuNode::slider("Chest Thickness", &self.chest_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::ChestThickness, delta)
			}),
			MenuNode::slider("Hip Thickness", &self.hip_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::HipThickness, delta)
			}),
			MenuNode::slider("Leg Thickness", &self.leg_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::LegThickness, delta)
			}),
			MenuNode::slider("Buttocks Thickness", &self.buttocks_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::ButtocksThickness, delta)
			}),
			MenuNode::slider("Waist Thickness", &self.waist_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::WaistThickness, delta)
			}),
			MenuNode::slider("Lower Trunk Thickness", &self.lower_trunk_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::LowerTrunkThickness, delta)
			}),
			MenuNode::slider("Arm Length", &self.arm_length, |delta| {
				MenuEvent::SliderDelta(CharacterField::ArmLength, delta)
			}),
			MenuNode::slider("Arm Thickness", &self.arm_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::ArmThickness, delta)
			}),
			MenuNode::slider("Leg Length", &self.leg_length, |delta| {
				MenuEvent::SliderDelta(CharacterField::LegLength, delta)
			}),
		])
	}
}

impl BraidmanHeadFeatureSliders {
	pub fn from_config(sliders: BraidmanSliders) -> Self {
		Self {
			eye_width: slider(sliders.eye_width, 0.8, 1.2, 0.05),
			eye_height: slider(sliders.eye_height, 0.8, 1.2, 0.05),
			eye_tilt: slider(sliders.eye_tilt, -30.0, 30.0, 0.5),
			nose_width: slider(sliders.nose_width, 0.8, 1.2, 0.05),
			nose_height: slider(sliders.nose_height, 0.8, 1.2, 0.05),
			mouth_width: slider(sliders.mouth_width, 0.8, 1.2, 0.05),
			mouth_height: slider(sliders.mouth_height, 0.8, 1.2, 0.05),
			ear_width: slider(sliders.ear_width, 0.8, 1.2, 0.05),
			ear_height: slider(sliders.ear_height, 0.8, 1.2, 0.05),
		}
	}

	pub fn write_config(&self, sliders: &mut BraidmanSliders) {
		sliders.eye_width = self.eye_width.value;
		sliders.eye_height = self.eye_height.value;
		sliders.eye_tilt = self.eye_tilt.value;
		sliders.nose_width = self.nose_width.value;
		sliders.nose_height = self.nose_height.value;
		sliders.mouth_width = self.mouth_width.value;
		sliders.mouth_height = self.mouth_height.value;
		sliders.ear_width = self.ear_width.value;
		sliders.ear_height = self.ear_height.value;
	}
}

impl From<&BraidmanConfig> for BraidmanMenu {
	fn from(config: &BraidmanConfig) -> Self {
		Self {
			presets: Section::new(
				"Presets",
				BraidmanPresetsMenu {
					gender: SingleSelect::new(config.gender),
					build: SingleSelect::new(config.build),
				},
			),
			body: Section::new(
				"Body",
				BraidmanBodyMenu {
					body: AssetSingleSelect::new(config.body).with_camera_focus(BODY_FOCUS),
					sliders: BraidmanBodyProportionSliders::from_config(config.sliders),
					color: SwatchSingleSelect::new(config.colors.body),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			head_features: Section::new(
				"Head & Features",
				BraidmanHeadFeaturesMenu {
					head: AssetSingleSelect::new(config.head).with_camera_focus(HEAD_ROOT_FOCUS),
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					nose: AssetSingleSelect::new(config.nose).with_camera_focus(NOSE_FOCUS),
					mouth: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					ear: AssetSingleSelect::new(config.ear).with_camera_focus(EAR_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
					feature_sliders: BraidmanHeadFeatureSliders::from_config(config.sliders),
					body_color: config.colors.body,
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

impl From<&BraidmanMenu> for BraidmanConfig {
	fn from(menu: &BraidmanMenu) -> Self {
		let body_color = menu.body.value.color.value;
		Self {
			gender: menu.presets.value.gender.value,
			build: menu.presets.value.build.value,
			body: menu.body.value.body.value,
			head: menu.head_features.value.head.value,
			eye: menu.head_features.value.eye.value,
			nose: menu.head_features.value.nose.value,
			mouth: menu.head_features.value.mouth.value,
			ear: menu.head_features.value.ear.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: BraidmanColors {
				body: body_color,
				head: body_color,
				eyes: menu.head_features.value.eye_color.value,
				nose: body_color,
				mouth: menu.head_features.value.mouth_color.value,
				ears: body_color,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
			sliders: {
				let mut sliders = BraidmanSliders::default();
				menu.body.value.sliders.write_config(&mut sliders);
				menu.head_features.value.feature_sliders.write_config(&mut sliders);
				sliders.clamped()
			},
		}
	}
}

impl MenuComponent<MenuEvent> for BraidmanPresetsMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::cycle("Gender", &self.gender, |delta| {
				MenuEvent::Cycle(CharacterField::Gender, delta)
			}),
			MenuNode::cycle("Build", &self.build, |delta| {
				MenuEvent::Cycle(CharacterField::Build, delta)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BraidmanBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Body Mesh",
				&self.body,
				PreviewColor::of(self.color.value),
				|value| MenuEvent::SetAsset(CharacterField::BodyMesh, AssetValue::Body(value)),
			),
			self.sliders.menu_node(),
			MenuNode::swatch("Body Color", &self.color, |color| {
				MenuEvent::SetSwatch(CharacterField::BodyColor, SwatchValue::Item(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BraidmanHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		let base = PreviewColor::of(self.body_color);
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, base, |value| {
				MenuEvent::SetAsset(CharacterField::HeadMesh, AssetValue::Head(value))
			}),
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::slider("Eye Width", &self.feature_sliders.eye_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::EyeWidth, delta)
			}),
			MenuNode::slider("Eye Height", &self.feature_sliders.eye_height, |delta| {
				MenuEvent::SliderDelta(CharacterField::EyeHeight, delta)
			}),
			MenuNode::slider("Eye Tilt", &self.feature_sliders.eye_tilt, |delta| {
				MenuEvent::SliderDelta(CharacterField::EyeTilt, delta)
			}),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::EyeColor, SwatchValue::Item(color))
			}),
			MenuNode::asset_grid("Nose", &self.nose, base, |value| {
				MenuEvent::SetAsset(CharacterField::Nose, AssetValue::Nose(value))
			}),
			MenuNode::slider("Nose Width", &self.feature_sliders.nose_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::NoseWidth, delta)
			}),
			MenuNode::slider("Nose Height", &self.feature_sliders.nose_height, |delta| {
				MenuEvent::SliderDelta(CharacterField::NoseHeight, delta)
			}),
			MenuNode::asset_grid(
				"Mouth",
				&self.mouth,
				PreviewColor::of(self.mouth_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Mouth, AssetValue::Mouth(value)),
			),
			MenuNode::slider("Mouth Width", &self.feature_sliders.mouth_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::MouthWidth, delta)
			}),
			MenuNode::slider("Mouth Height", &self.feature_sliders.mouth_height, |delta| {
				MenuEvent::SliderDelta(CharacterField::MouthHeight, delta)
			}),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(CharacterField::MouthColor, SwatchValue::Item(color))
			}),
			MenuNode::asset_grid("Ears", &self.ear, base, |value| {
				MenuEvent::SetAsset(CharacterField::Ear, AssetValue::Ear(value))
			}),
			MenuNode::slider("Ear Width", &self.feature_sliders.ear_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::EarWidth, delta)
			}),
			MenuNode::slider("Ear Height", &self.feature_sliders.ear_height, |delta| {
				MenuEvent::SliderDelta(CharacterField::EarHeight, delta)
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for BraidmanMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.presets.label, self.presets.value.menu_node()),
			MenuNode::section(self.body.label, self.body.value.menu_node()),
			MenuNode::section(self.head_features.label, self.head_features.value.menu_node()),
			MenuNode::section(self.hair.label, self.hair.value.menu_node()),
			MenuNode::section(self.clothing.label, self.clothing.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl BraidmanMenu {
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
			CharacterField::BodyMesh => self.body.value.body.camera_focus,
			CharacterField::HeadMesh => self.head_features.value.head.camera_focus,
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

impl Default for BraidmanMenu {
	fn default() -> Self {
		Self::from(&BraidmanConfig::default_preview())
	}
}
