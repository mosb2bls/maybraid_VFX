use character_ui_menu::{CameraFocus, MenuComponent, MenuNode, Section, SwatchSingleSelect};
use characters::{
	species::mistler::{MistlerBodyColor, MistlerColors, MistlerConfig},
	ConceptAnimation,
};

use crate::{
	event::{CharacterField, MenuEvent, SwatchValue},
	focus::MISTLER_BODY_FOCUS,
	shared::AnimationMenu,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MistlerBodyMenu {
	pub body: SwatchSingleSelect<MistlerBodyColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MistlerMenu {
	pub body: Section<MistlerBodyMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&MistlerConfig> for MistlerMenu {
	fn from(config: &MistlerConfig) -> Self {
		Self {
			body: Section::new(
				"Body",
				MistlerBodyMenu {
					body: SwatchSingleSelect::new(config.colors.body)
						.with_camera_focus(MISTLER_BODY_FOCUS),
				},
			)
			.with_camera_focus(MISTLER_BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(MISTLER_BODY_FOCUS)),
		}
	}
}

impl From<&MistlerMenu> for MistlerConfig {
	fn from(menu: &MistlerMenu) -> Self {
		Self { colors: MistlerColors { body: menu.body.value.body.value } }
	}
}

impl MenuComponent<MenuEvent> for MistlerBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::swatch("Body Color", &self.body, |color| {
			MenuEvent::SetSwatch(CharacterField::MistlerBodyColor, SwatchValue::MistlerBody(color))
		})
	}
}

impl MenuComponent<MenuEvent> for MistlerMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.body.label, self.body.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl MistlerMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = animation;
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::MistlerBodyColor => self.body.value.body.camera_focus,
			CharacterField::Animation => Some(MISTLER_BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for MistlerMenu {
	fn default() -> Self {
		Self::from(&MistlerConfig::default_preview())
	}
}
