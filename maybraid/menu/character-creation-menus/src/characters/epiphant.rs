use bevy_math::Vec3;
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, FocusRig, IdentifiedAsset, MenuComponent, MenuNode,
	PreviewColor, Section, SingleSelect, SwatchSingleSelect,
};
use characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		common::EyeMesh,
		epiphant::{
			sliders::EpiphantSliders, EpiphantBodyMesh, EpiphantColor, EpiphantColors,
			EpiphantConfig, EpiphantEarMesh, EpiphantHeadMesh, EpiphantNoseMesh,
		},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{EYE_FOCUS, NOSE_FOCUS},
};

/// Epiphant quadruped body framing; also the species' default camera focus.
pub const BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"back_ridge",
	Vec3::new(2.0, 1.0, 4.0),
	Vec3::new(1.0, 0.0, 0.0),
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpiphantAnimationClip {
	Still,
	Run,
	Gallop,
}

impl character_ui_menu::ListValues for EpiphantAnimationClip {
	fn values() -> &'static [Self] {
		&[Self::Still, Self::Run, Self::Gallop]
	}
}

impl character_ui_menu::LabelOption for EpiphantAnimationClip {
	fn label(&self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Run => "run",
			Self::Gallop => "gallop",
		}
	}
}

impl character_ui_menu::AssetOption for EpiphantAnimationClip {
	fn asset(&self) -> IdentifiedAsset {
		let label = match self {
			Self::Still => "still",
			Self::Run => "run",
			Self::Gallop => "gallop",
		};
		IdentifiedAsset::new(label, label, "")
	}
}

impl From<EpiphantAnimationClip> for ConceptAnimation {
	fn from(value: EpiphantAnimationClip) -> Self {
		match value {
			EpiphantAnimationClip::Still => ConceptAnimation::Still,
			EpiphantAnimationClip::Run => ConceptAnimation::Run,
			EpiphantAnimationClip::Gallop => ConceptAnimation::Gallop,
		}
	}
}

impl From<ConceptAnimation> for EpiphantAnimationClip {
	fn from(value: ConceptAnimation) -> Self {
		match value {
			ConceptAnimation::Run => Self::Run,
			ConceptAnimation::Gallop => Self::Gallop,
			_ => Self::Still,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantAnimationMenu {
	pub clip: AssetSingleSelect<EpiphantAnimationClip>,
}

impl EpiphantAnimationMenu {
	pub fn new() -> Self {
		Self {
			clip: AssetSingleSelect::new(EpiphantAnimationClip::Still)
				.with_camera_focus(BODY_FOCUS),
		}
	}
}

impl MenuComponent<MenuEvent> for EpiphantAnimationMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::asset_grid("Clip", &self.clip, PreviewColor::WHITE, |value| {
			MenuEvent::SetAsset(
				CharacterField::Animation,
				AssetValue::Animation(ConceptAnimation::from(value)),
			)
		})
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantPresetsMenu {
	pub gender: SingleSelect<GenderPreset>,
	pub build: SingleSelect<BuildPreset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantBodyProportionSliders {
	pub shoulder_width: character_ui_menu::Slider,
	pub hip_width: character_ui_menu::Slider,
	pub chest_thickness: character_ui_menu::Slider,
	pub hip_thickness: character_ui_menu::Slider,
	pub leg_thickness: character_ui_menu::Slider,
	pub buttocks_thickness: character_ui_menu::Slider,
	pub waist_thickness: character_ui_menu::Slider,
	pub lower_trunk_thickness: character_ui_menu::Slider,
	pub arm_length: character_ui_menu::Slider,
	pub arm_thickness: character_ui_menu::Slider,
	pub leg_length: character_ui_menu::Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantHeadFeatureSliders {
	pub eye_width: character_ui_menu::Slider,
	pub eye_height: character_ui_menu::Slider,
	pub eye_tilt: character_ui_menu::Slider,
	pub ear_width: character_ui_menu::Slider,
	pub ear_height: character_ui_menu::Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantBodyMenu {
	pub body: AssetSingleSelect<EpiphantBodyMesh>,
	pub sliders: EpiphantBodyProportionSliders,
	pub color: SwatchSingleSelect<EpiphantColor>,
	pub tail_color: SwatchSingleSelect<EpiphantColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub nose: AssetSingleSelect<EpiphantNoseMesh>,
	pub eye_color: SwatchSingleSelect<EpiphantColor>,
	pub nose_color: SwatchSingleSelect<EpiphantColor>,
	pub ear_color: SwatchSingleSelect<EpiphantColor>,
	pub feature_sliders: EpiphantHeadFeatureSliders,
	pub body_color: EpiphantColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EpiphantMenu {
	pub presets: Section<EpiphantPresetsMenu>,
	pub body: Section<EpiphantBodyMenu>,
	pub head_features: Section<EpiphantHeadFeaturesMenu>,
	pub animation: Section<EpiphantAnimationMenu>,
}

fn slider(value: f32, min: f32, max: f32, step: f32) -> character_ui_menu::Slider {
	character_ui_menu::Slider::new(value, min, max, step)
}

impl EpiphantBodyProportionSliders {
	pub fn from_config(sliders: EpiphantSliders) -> Self {
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

	pub fn write_config(&self, sliders: &mut EpiphantSliders) {
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

impl MenuComponent<MenuEvent> for EpiphantBodyProportionSliders {
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
			MenuNode::slider("Front Leg Length", &self.arm_length, |delta| {
				MenuEvent::SliderDelta(CharacterField::ArmLength, delta)
			}),
			MenuNode::slider("Front Leg Thickness", &self.arm_thickness, |delta| {
				MenuEvent::SliderDelta(CharacterField::ArmThickness, delta)
			}),
			MenuNode::slider("Hind Leg Length", &self.leg_length, |delta| {
				MenuEvent::SliderDelta(CharacterField::LegLength, delta)
			}),
		])
	}
}

impl EpiphantHeadFeatureSliders {
	pub fn from_config(sliders: EpiphantSliders) -> Self {
		Self {
			eye_width: slider(sliders.eye_width, 0.8, 1.2, 0.05),
			eye_height: slider(sliders.eye_height, 0.8, 1.2, 0.05),
			eye_tilt: slider(sliders.eye_tilt, -30.0, 30.0, 0.5),
			ear_width: slider(sliders.ear_width, 0.8, 1.2, 0.05),
			ear_height: slider(sliders.ear_height, 0.8, 1.2, 0.05),
		}
	}

	pub fn write_config(&self, sliders: &mut EpiphantSliders) {
		sliders.eye_width = self.eye_width.value;
		sliders.eye_height = self.eye_height.value;
		sliders.eye_tilt = self.eye_tilt.value;
		sliders.ear_width = self.ear_width.value;
		sliders.ear_height = self.ear_height.value;
	}
}

impl MenuComponent<MenuEvent> for EpiphantHeadFeatureSliders {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::slider("Eye Width", &self.eye_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::EyeWidth, delta)
			}),
			MenuNode::slider("Eye Height", &self.eye_height, |delta| {
				MenuEvent::SliderDelta(CharacterField::EyeHeight, delta)
			}),
			MenuNode::slider("Eye Tilt", &self.eye_tilt, |delta| {
				MenuEvent::SliderDelta(CharacterField::EyeTilt, delta)
			}),
			MenuNode::slider("Ear Width", &self.ear_width, |delta| {
				MenuEvent::SliderDelta(CharacterField::EarWidth, delta)
			}),
			MenuNode::slider("Ear Height", &self.ear_height, |delta| {
				MenuEvent::SliderDelta(CharacterField::EarHeight, delta)
			}),
		])
	}
}

impl From<&EpiphantConfig> for EpiphantMenu {
	fn from(config: &EpiphantConfig) -> Self {
		Self {
			presets: Section::new(
				"Presets",
				EpiphantPresetsMenu {
					gender: SingleSelect::new(config.gender),
					build: SingleSelect::new(config.build),
				},
			),
			body: Section::new(
				"Body",
				EpiphantBodyMenu {
					body: AssetSingleSelect::new(config.body).with_camera_focus(BODY_FOCUS),
					sliders: EpiphantBodyProportionSliders::from_config(config.sliders),
					color: SwatchSingleSelect::new(config.colors.body),
					tail_color: SwatchSingleSelect::new(config.colors.tail),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			head_features: Section::new(
				"Head & Features",
				EpiphantHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					nose: AssetSingleSelect::new(config.nose).with_camera_focus(NOSE_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					nose_color: SwatchSingleSelect::new(config.colors.nose),
					ear_color: SwatchSingleSelect::new(config.colors.ears),
					feature_sliders: EpiphantHeadFeatureSliders::from_config(config.sliders),
					body_color: config.colors.body,
				},
			),
			animation: Section::new("Animation", EpiphantAnimationMenu::new()),
		}
	}
}

impl From<&EpiphantMenu> for EpiphantConfig {
	fn from(menu: &EpiphantMenu) -> Self {
		let body_color = menu.body.value.color.value;
		Self {
			gender: menu.presets.value.gender.value,
			build: menu.presets.value.build.value,
			body: menu.body.value.body.value,
			head: EpiphantHeadMesh::Meerkat,
			ear: EpiphantEarMesh::Epiphant,
			nose: menu.head_features.value.nose.value,
			eye: menu.head_features.value.eye.value,
			colors: EpiphantColors {
				body: body_color,
				head: body_color,
				eyes: menu.head_features.value.eye_color.value,
				ears: menu.head_features.value.ear_color.value,
				nose: menu.head_features.value.nose_color.value,
				tail: menu.body.value.tail_color.value,
			},
			sliders: {
				let mut sliders = EpiphantSliders::default();
				menu.body.value.sliders.write_config(&mut sliders);
				menu.head_features.value.feature_sliders.write_config(&mut sliders);
				sliders.clamped()
			},
		}
	}
}

impl MenuComponent<MenuEvent> for EpiphantPresetsMenu {
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

impl MenuComponent<MenuEvent> for EpiphantBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Body Mesh",
				&self.body,
				PreviewColor::of(self.color.value),
				|value| {
					MenuEvent::SetAsset(
						CharacterField::EpiphantBody,
						AssetValue::EpiphantBody(value),
					)
				},
			),
			self.sliders.menu_node(),
			MenuNode::swatch("Body Color", &self.color, |color| {
				MenuEvent::SetSwatch(CharacterField::BodyColor, SwatchValue::Epiphant(color))
			}),
			MenuNode::swatch("Tail Color", &self.tail_color, |color| {
				MenuEvent::SetSwatch(CharacterField::TailColor, SwatchValue::Epiphant(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for EpiphantHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Eyes",
				&self.eye,
				PreviewColor::of(self.eye_color.value),
				|value| MenuEvent::SetAsset(CharacterField::Eye, AssetValue::Eye(value)),
			),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::EyeColor, SwatchValue::Epiphant(color))
			}),
			MenuNode::asset_grid(
				"Trunk",
				&self.nose,
				PreviewColor::of(self.nose_color.value),
				|value| {
					MenuEvent::SetAsset(
						CharacterField::EpiphantNose,
						AssetValue::EpiphantNose(value),
					)
				},
			),
			MenuNode::swatch("Trunk Color", &self.nose_color, |color| {
				MenuEvent::SetSwatch(CharacterField::NoseColor, SwatchValue::Epiphant(color))
			}),
			MenuNode::swatch("Ear Color", &self.ear_color, |color| {
				MenuEvent::SetSwatch(CharacterField::EarColor, SwatchValue::Epiphant(color))
			}),
			self.feature_sliders.menu_node(),
		])
	}
}

impl MenuComponent<MenuEvent> for EpiphantMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.presets.label, self.presets.value.menu_node()),
			MenuNode::section(self.body.label, self.body.value.menu_node()),
			MenuNode::section(self.head_features.label, self.head_features.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl EpiphantMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = EpiphantAnimationClip::from(animation);
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value.into()
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::EpiphantBody => self.body.value.body.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::EpiphantNose => self.head_features.value.nose.camera_focus,
			CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for EpiphantMenu {
	fn default() -> Self {
		Self::from(&EpiphantConfig::default_preview())
	}
}
