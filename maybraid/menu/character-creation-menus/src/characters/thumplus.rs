use character_ui_menu::{CameraFocus, MenuComponent, MenuNode, Section, SwatchSingleSelect};
use characters::{
	species::thumplus::{ThumplusBodyColor, ThumplusColors, ThumplusConfig},
	ConceptAnimation,
};

use crate::{
	event::{CharacterField, MenuEvent, SwatchValue},
	focus::THUMPLUS_BODY_FOCUS,
	shared::AnimationMenu,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ThumplusBodyMenu {
	pub body: SwatchSingleSelect<ThumplusBodyColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThumplusMenu {
	pub body: Section<ThumplusBodyMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&ThumplusConfig> for ThumplusMenu {
	fn from(config: &ThumplusConfig) -> Self {
		Self {
			body: Section::new(
				"Body",
				ThumplusBodyMenu {
					body: SwatchSingleSelect::new(config.colors.body)
						.with_camera_focus(THUMPLUS_BODY_FOCUS),
				},
			)
			.with_camera_focus(THUMPLUS_BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(THUMPLUS_BODY_FOCUS)),
		}
	}
}

impl From<&ThumplusMenu> for ThumplusConfig {
	fn from(menu: &ThumplusMenu) -> Self {
		Self { colors: ThumplusColors { body: menu.body.value.body.value } }
	}
}

impl MenuComponent<MenuEvent> for ThumplusBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::swatch("Body Color", &self.body, |color| {
			MenuEvent::SetSwatch(
				CharacterField::ThumplusBodyColor,
				SwatchValue::ThumplusBody(color),
			)
		})
	}
}

impl MenuComponent<MenuEvent> for ThumplusMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.body.label, self.body.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl ThumplusMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = animation;
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::ThumplusBodyColor => self.body.value.body.camera_focus,
			CharacterField::Animation => Some(THUMPLUS_BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for ThumplusMenu {
	fn default() -> Self {
		Self::from(&ThumplusConfig::default_preview())
	}
}
