use bevy_math::Vec3;
use character_items::ItemColor;
use character_ui_menu::{
	AssetSingleSelect, CameraFocus, FocusRig, IdentifiedAsset, MenuComponent, MenuNode,
	PreviewColor, Section, SingleSelect, SwatchSingleSelect,
};
use characters::{
	presets::{BuildPreset, GenderPreset},
	species::sonyak::{
		sliders::SonyakSliders, SonyakBodyMesh, SonyakColors, SonyakConfig, SonyakMouthMesh,
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::MOUTH_FOCUS,
};

/// Sonyak quadruped body framing; also the species' default camera focus.
pub const BODY_FOCUS: CameraFocus = CameraFocus::new(
	FocusRig::Body,
	"back_ridge",
	Vec3::new(2.0, 1.0, 4.0),
	Vec3::new(1.0, 0.0, 0.0),
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SonyakAnimationClip {
	Still,
	Run,
	Gallop,
}

impl character_ui_menu::ListValues for SonyakAnimationClip {
	fn values() -> &'static [Self] {
		&[Self::Still, Self::Run, Self::Gallop]
	}
}

impl character_ui_menu::LabelOption for SonyakAnimationClip {
	fn label(&self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Run => "run",
			Self::Gallop => "gallop",
		}
	}
}

impl character_ui_menu::AssetOption for SonyakAnimationClip {
	fn asset(&self) -> IdentifiedAsset {
		let label = match self {
			Self::Still => "still",
			Self::Run => "run",
			Self::Gallop => "gallop",
		};
		IdentifiedAsset::new(label, label, "")
	}
}

impl From<SonyakAnimationClip> for ConceptAnimation {
	fn from(value: SonyakAnimationClip) -> Self {
		match value {
			SonyakAnimationClip::Still => ConceptAnimation::Still,
			SonyakAnimationClip::Run => ConceptAnimation::Run,
			SonyakAnimationClip::Gallop => ConceptAnimation::Gallop,
		}
	}
}

impl From<ConceptAnimation> for SonyakAnimationClip {
	fn from(value: ConceptAnimation) -> Self {
		match value {
			ConceptAnimation::Run => Self::Run,
			ConceptAnimation::Gallop => Self::Gallop,
			_ => Self::Still,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct SonyakAnimationMenu {
	pub clip: AssetSingleSelect<SonyakAnimationClip>,
}

impl SonyakAnimationMenu {
	pub fn new() -> Self {
		Self {
			clip: AssetSingleSelect::new(SonyakAnimationClip::Still).with_camera_focus(BODY_FOCUS),
		}
	}
}

impl MenuComponent<MenuEvent> for SonyakAnimationMenu {
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
pub struct SonyakPresetsMenu {
	pub gender: SingleSelect<GenderPreset>,
	pub build: SingleSelect<BuildPreset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SonyakBodyProportionSliders {
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
pub struct SonyakHeadFeatureSliders {
	pub eye_width: character_ui_menu::Slider,
	pub eye_height: character_ui_menu::Slider,
	pub eye_tilt: character_ui_menu::Slider,
	pub ear_width: character_ui_menu::Slider,
	pub ear_height: character_ui_menu::Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SonyakBodyMenu {
	pub body: AssetSingleSelect<SonyakBodyMesh>,
	pub sliders: SonyakBodyProportionSliders,
	pub color: SwatchSingleSelect<ItemColor>,
	pub tail_color: SwatchSingleSelect<ItemColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SonyakHeadFeaturesMenu {
	pub snout: AssetSingleSelect<SonyakMouthMesh>,
	pub eye_color: SwatchSingleSelect<ItemColor>,
	pub mouth_color: SwatchSingleSelect<ItemColor>,
	pub hair_color: SwatchSingleSelect<ItemColor>,
	pub feature_sliders: SonyakHeadFeatureSliders,
	pub body_color: ItemColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SonyakMenu {
	pub presets: Section<SonyakPresetsMenu>,
	pub body: Section<SonyakBodyMenu>,
	pub head_features: Section<SonyakHeadFeaturesMenu>,
	pub animation: Section<SonyakAnimationMenu>,
}

fn slider(value: f32, min: f32, max: f32, step: f32) -> character_ui_menu::Slider {
	character_ui_menu::Slider::new(value, min, max, step)
}

impl SonyakBodyProportionSliders {
	pub fn from_config(sliders: SonyakSliders) -> Self {
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

	pub fn write_config(&self, sliders: &mut SonyakSliders) {
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

impl MenuComponent<MenuEvent> for SonyakBodyProportionSliders {
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

impl SonyakHeadFeatureSliders {
	pub fn from_config(sliders: SonyakSliders) -> Self {
		Self {
			eye_width: slider(sliders.eye_width, 0.8, 1.2, 0.05),
			eye_height: slider(sliders.eye_height, 0.8, 1.2, 0.05),
			eye_tilt: slider(sliders.eye_tilt, -30.0, 30.0, 0.5),
			ear_width: slider(sliders.ear_width, 0.8, 1.2, 0.05),
			ear_height: slider(sliders.ear_height, 0.8, 1.2, 0.05),
		}
	}

	pub fn write_config(&self, sliders: &mut SonyakSliders) {
		sliders.eye_width = self.eye_width.value;
		sliders.eye_height = self.eye_height.value;
		sliders.eye_tilt = self.eye_tilt.value;
		sliders.ear_width = self.ear_width.value;
		sliders.ear_height = self.ear_height.value;
	}
}

impl MenuComponent<MenuEvent> for SonyakHeadFeatureSliders {
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

impl From<&SonyakConfig> for SonyakMenu {
	fn from(config: &SonyakConfig) -> Self {
		Self {
			presets: Section::new(
				"Presets",
				SonyakPresetsMenu {
					gender: SingleSelect::new(config.gender),
					build: SingleSelect::new(config.build),
				},
			),
			body: Section::new(
				"Body",
				SonyakBodyMenu {
					body: AssetSingleSelect::new(SonyakBodyMesh::Gumbus)
						.with_camera_focus(BODY_FOCUS),
					sliders: SonyakBodyProportionSliders::from_config(config.sliders),
					color: SwatchSingleSelect::new(config.colors.body),
					tail_color: SwatchSingleSelect::new(config.colors.tail),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			head_features: Section::new(
				"Head & Features",
				SonyakHeadFeaturesMenu {
					snout: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
					hair_color: SwatchSingleSelect::new(config.colors.hair),
					feature_sliders: SonyakHeadFeatureSliders::from_config(config.sliders),
					body_color: config.colors.body,
				},
			),
			animation: Section::new("Animation", SonyakAnimationMenu::new()),
		}
	}
}

impl From<&SonyakMenu> for SonyakConfig {
	fn from(menu: &SonyakMenu) -> Self {
		let body_color = menu.body.value.color.value;
		Self {
			gender: menu.presets.value.gender.value,
			build: menu.presets.value.build.value,
			mouth: menu.head_features.value.snout.value,
			colors: SonyakColors {
				body: body_color,
				head: body_color,
				eyes: menu.head_features.value.eye_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				tail: menu.body.value.tail_color.value,
				hair: menu.head_features.value.hair_color.value,
			},
			sliders: {
				let mut sliders = SonyakSliders::default();
				menu.body.value.sliders.write_config(&mut sliders);
				menu.head_features.value.feature_sliders.write_config(&mut sliders);
				sliders.clamped()
			},
		}
	}
}

impl MenuComponent<MenuEvent> for SonyakPresetsMenu {
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

impl MenuComponent<MenuEvent> for SonyakBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::asset_grid(
				"Body Mesh",
				&self.body,
				PreviewColor::of(self.color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::SonyakBody, AssetValue::SonyakBody(value))
				},
			),
			self.sliders.menu_node(),
			MenuNode::swatch("Body Color", &self.color, |color| {
				MenuEvent::SetSwatch(CharacterField::BodyColor, SwatchValue::Item(color))
			}),
			MenuNode::swatch("Tail Color", &self.tail_color, |color| {
				MenuEvent::SetSwatch(CharacterField::TailColor, SwatchValue::Item(color))
			}),
		])
	}
}

impl MenuComponent<MenuEvent> for SonyakHeadFeaturesMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::EyeColor, SwatchValue::Item(color))
			}),
			MenuNode::asset_grid(
				"Snout",
				&self.snout,
				PreviewColor::of(self.mouth_color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::SonyakMouth, AssetValue::SonyakMouth(value))
				},
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(CharacterField::MouthColor, SwatchValue::Item(color))
			}),
			MenuNode::swatch("Hair Color", &self.hair_color, |color| {
				MenuEvent::SetSwatch(CharacterField::HairColor, SwatchValue::Item(color))
			}),
			self.feature_sliders.menu_node(),
		])
	}
}

impl MenuComponent<MenuEvent> for SonyakMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.presets.label, self.presets.value.menu_node()),
			MenuNode::section(self.body.label, self.body.value.menu_node()),
			MenuNode::section(self.head_features.label, self.head_features.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl SonyakMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = SonyakAnimationClip::from(animation);
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value.into()
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::SonyakBody => self.body.value.body.camera_focus,
			CharacterField::SonyakMouth => self.head_features.value.snout.camera_focus,
			CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for SonyakMenu {
	fn default() -> Self {
		Self::from(&SonyakConfig::default_preview())
	}
}
