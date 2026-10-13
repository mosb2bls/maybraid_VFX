use character_ui_menu::{CameraFocus, MenuComponent, MenuNode, Section, SwatchSingleSelect};
use characters::{
	species::grener::{GrenerBodyColor, GrenerColors, GrenerConfig},
	ConceptAnimation,
};

use crate::{
	event::{CharacterField, MenuEvent, SwatchValue},
	focus::GRENER_BODY_FOCUS,
	shared::AnimationMenu,
};

#[derive(Clone, Debug, PartialEq)]
pub struct GrenerBodyMenu {
	pub body: SwatchSingleSelect<GrenerBodyColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GrenerMenu {
	pub body: Section<GrenerBodyMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&GrenerConfig> for GrenerMenu {
	fn from(config: &GrenerConfig) -> Self {
		Self {
			body: Section::new(
				"Body",
				GrenerBodyMenu {
					body: SwatchSingleSelect::new(config.colors.body)
						.with_camera_focus(GRENER_BODY_FOCUS),
				},
			)
			.with_camera_focus(GRENER_BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(GRENER_BODY_FOCUS)),
		}
	}
}

impl From<&GrenerMenu> for GrenerConfig {
	fn from(menu: &GrenerMenu) -> Self {
		Self { colors: GrenerColors { body: menu.body.value.body.value } }
	}
}

impl MenuComponent<MenuEvent> for GrenerBodyMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::swatch("Body Color", &self.body, |color| {
			MenuEvent::SetSwatch(CharacterField::GrenerBodyColor, SwatchValue::GrenerBody(color))
		})
	}
}

impl MenuComponent<MenuEvent> for GrenerMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		MenuNode::fragment([
			MenuNode::section(self.body.label, self.body.value.menu_node()),
			MenuNode::section(self.animation.label, self.animation.value.menu_node()),
		])
	}
}

impl GrenerMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = animation;
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::GrenerBodyColor => self.body.value.body.camera_focus,
			CharacterField::Animation => Some(GRENER_BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for GrenerMenu {
	fn default() -> Self {
		Self::from(&GrenerConfig::default_preview())
	}
}
