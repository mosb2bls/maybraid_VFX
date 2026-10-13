use bevy_math::Vec3;
use character_items::{ClothingMesh, ItemColor};
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, FocusRig, MenuComponent, MenuNode, PreviewColor, Section,
	SingleSelect, Slider, SwatchSingleSelect,
};
use characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		common::{EyeMesh, MouthMesh, NoseMesh},
		tuberwaber::{
			sliders::TuberwaberSliders, TuberwaberBodyMesh, TuberwaberColor, TuberwaberColors,
			TuberwaberConfig, TuberwaberHeadMesh,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

pub const BODY_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Body, "root", Vec3::new(-1.0, 1.0, 4.0), Vec3::new(2.0, 0.0, -2.0));

pub const HEAD_ROOT_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "root", Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.05, 0.0));

pub const CROWN_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "crown_socket", Vec3::new(0.0, 0.15, 1.0), Vec3::ZERO);

pub const EYE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "eye_socket.L", Vec3::new(0.0, 0.0, 0.35), Vec3::ZERO);

pub const NOSE_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "nose_socket", Vec3::new(0.0, 0.0, 0.25), Vec3::ZERO);

pub const MOUTH_FOCUS: CameraFocus =
	CameraFocus::new(FocusRig::Head, "mouth_socket", Vec3::new(0.0, 0.0, 0.25), Vec3::ZERO);

#[derive(Clone, Debug, PartialEq)]
pub struct TuberwaberPresetsMenu {
	pub gender: SingleSelect<GenderPreset>,
	pub build: SingleSelect<BuildPreset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TuberwaberBodyProportionSliders {
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
pub struct TuberwaberHeadFeatureSliders {
	pub eye_width: Slider,
	pub eye_height: Slider,
	pub eye_tilt: Slider,
	pub nose_width: Slider,
	pub nose_height: Slider,
	pub mouth_width: Slider,
	pub mouth_height: Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TuberwaberBodyMenu {
	pub body: AssetSingleSelect<TuberwaberBodyMesh>,
	pub sliders: TuberwaberBodyProportionSliders,
	pub color: SwatchSingleSelect<TuberwaberColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TuberwaberHeadFeaturesMenu {
	pub head: AssetSingleSelect<TuberwaberHeadMesh>,
	pub eye: AssetSingleSelect<EyeMesh>,
	pub nose: AssetSingleSelect<NoseMesh>,
	pub mouth: AssetSingleSelect<MouthMesh>,
	pub eye_color: SwatchSingleSelect<TuberwaberColor>,
	pub mouth_color: SwatchSingleSelect<TuberwaberColor>,
	pub horn_color: SwatchSingleSelect<TuberwaberColor>,
	pub feature_sliders: TuberwaberHeadFeatureSliders,
	pub body_color: TuberwaberColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TuberwaberMenu {
	pub presets: Section<TuberwaberPresetsMenu>,
	pub body: Section<TuberwaberBodyMenu>,
	pub head_features: Section<TuberwaberHeadFeaturesMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

fn slider(value: f32, min: f32, max: f32, step: f32) -> Slider {
	Slider::new(value, min, max, step)
}

impl TuberwaberBodyProportionSliders {
	pub fn from_config(sliders: TuberwaberSliders) -> Self {
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

	pub fn write_config(&self, sliders: &mut TuberwaberSliders) {
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

impl MenuComponent<MenuEvent> for TuberwaberBodyProportionSliders {
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

impl TuberwaberHeadFeatureSliders {
	pub fn from_config(sliders: TuberwaberSliders) -> Self {
		Self {
			eye_width: slider(sliders.eye_width, 0.8, 1.2, 0.05),
			eye_height: slider(sliders.eye_height, 0.8, 1.2, 0.05),
			eye_tilt: slider(sliders.eye_tilt, -30.0, 30.0, 0.5),
			nose_width: slider(sliders.nose_width, 0.8, 1.2, 0.05),
			nose_height: slider(sliders.nose_height, 0.8, 1.2, 0.05),
			mouth_width: slider(sliders.mouth_width, 0.8, 1.2, 0.05),
			mouth_height: slider(sliders.mouth_height, 0.8, 1.2, 0.05),
		}
	}

	pub fn write_config(&self, sliders: &mut TuberwaberSliders) {
		sliders.eye_width = self.eye_width.value;
		sliders.eye_height = self.eye_height.value;
		sliders.eye_tilt = self.eye_tilt.value;
		sliders.nose_width = self.nose_width.value;
		sliders.nose_height = self.nose_height.value;
		sliders.mouth_width = self.mouth_width.value;
		sliders.mouth_height = self.mouth_height.value;
	}
}

impl From<&TuberwaberConfig> for TuberwaberMenu {
	fn from(config: &TuberwaberConfig) -> Self {
		Self {
			presets: Section::new(
				"Presets",
				TuberwaberPresetsMenu {
					gender: SingleSelect::new(config.gender),
					build: SingleSelect::new(config.build),
				},
			),
			body: Section::new(
				"Body",
				TuberwaberBodyMenu {
					body: AssetSingleSelect::new(config.body).with_camera_focus(BODY_FOCUS),
					sliders: TuberwaberBodyProportionSliders::from_config(config.sliders),
					color: SwatchSingleSelect::new(config.colors.body),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			head_features: Section::new(
				"Head & Features",
				TuberwaberHeadFeaturesMenu {
					head: AssetSingleSelect::new(config.head).with_camera_focus(HEAD_ROOT_FOCUS),
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					nose: AssetSingleSelect::new(config.nose).with_camera_focus(NOSE_FOCUS),
					mouth: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
					horn_color: SwatchSingleSelect::new(config.colors.horns),
					feature_sliders: TuberwaberHeadFeatureSliders::from_config(config.sliders),
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

impl From<&TuberwaberMenu> for TuberwaberConfig {
	fn from(menu: &TuberwaberMenu) -> Self {
		let body_color = menu.body.value.color.value;
		Self {
			gender: menu.presets.value.gender.value,
			build: menu.presets.value.build.value,
			body: menu.body.value.body.value,
			head: menu.head_features.value.head.value,
			eye: menu.head_features.value.eye.value,
			nose: menu.head_features.value.nose.value,
			mouth: menu.head_features.value.mouth.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: TuberwaberColors {
				body: body_color,
				head: body_color,
				eyes: menu.head_features.value.eye_color.value,
				nose: body_color,
				mouth: menu.head_features.value.mouth_color.value,
				horns: menu.head_features.value.horn_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing_material: menu.clothing.value.material.value,
				clothing_materials: menu.clothing.value.item_materials.clone(),
				clothing: menu.clothing.value.item_colors.clone(),
			},
			sliders: {
				let mut sliders = TuberwaberSliders::default();
				menu.body.value.sliders.write_config(&mut sliders);
				menu.head_features.value.feature_sliders.write_config(&mut sliders);
				sliders.clamped()
			},
		}
	}
}

impl MenuComponent<MenuEvent> for TuberwaberPresetsMenu {
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

impl MenuComponent<MenuEvent> for TuberwaberBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Body Mesh",
				&self.body,
				PreviewColor::of(self.color.value),
				|value| {
					MenuEvent::SetAsset(
						CharacterField::TuberwaberBody,
						AssetValue::TuberwaberBody(value),
					)
				},
			),
			self.sliders.menu_node(),
			MenuNode::swatch("Body Color", &self.color, |color| {
				MenuEvent::SetSwatch(CharacterField::BodyColor, SwatchValue::Tuberwaber(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for TuberwaberHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		let base = PreviewColor::of(self.body_color);
		MenuNode::fragment([
			MenuNode::asset_grid("Head", &self.head, base, |value| {
				MenuEvent::SetAsset(
					CharacterField::TuberwaberHead,
					AssetValue::TuberwaberHead(value),
				)
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
				MenuEvent::SetSwatch(CharacterField::EyeColor, SwatchValue::Tuberwaber(color))
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
				MenuEvent::SetSwatch(CharacterField::MouthColor, SwatchValue::Tuberwaber(color))
			}),
			MenuNode::swatch("Crown Color", &self.horn_color, |color| {
				MenuEvent::SetSwatch(CharacterField::HornColor, SwatchValue::Tuberwaber(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for TuberwaberMenu {
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

impl TuberwaberMenu {
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
			CharacterField::TuberwaberBody => self.body.value.body.camera_focus,
			CharacterField::TuberwaberHead => self.head_features.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::Nose => self.head_features.value.nose.camera_focus,
			CharacterField::Mouth => self.head_features.value.mouth.camera_focus,
			CharacterField::Hair | CharacterField::HornColor => Some(CROWN_FOCUS),
			CharacterField::Clothing(_)
			| CharacterField::ClothingMaterial(_)
			| CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for TuberwaberMenu {
	fn default() -> Self {
		Self::from(&TuberwaberConfig::default_preview())
	}
}
