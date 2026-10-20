//! Scope component and exclusive focus pick.

use bevy::prelude::*;
use maybraid_controllers_core::{InputSource, Routed, UiTarget};
use maybraid_input::{MenuNav, MenuNavImpulse};
use menu_components::{HudMenu, HudOverlayMenu, TextMenu};

/// Input scope. Nav from [`crate::dispatch::dispatch_menu_nav`] is delivered to
/// one focused descendant, not every child.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuController {
	pub focus: Option<UiTarget>,
}

impl MenuController {
	pub fn focused_entity(&self) -> Option<Entity> {
		self.focus.map(UiTarget::entity)
	}

	pub fn set_focus(&mut self, entity: Option<Entity>) {
		self.focus = entity.map(UiTarget::new);
	}

	/// Overlay, then any HUD list, then a text column.
	pub fn pick_focus(
		overlay: Option<Entity>,
		hud: Option<Entity>,
		text: Option<Entity>,
	) -> Option<Entity> {
		overlay.or(hud).or(text)
	}

	pub fn resolve(
		root: Entity,
		children: &Query<&Children>,
		overlays: &Query<Entity, With<HudOverlayMenu>>,
		hud_menus: &Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
		text_menus: &Query<Entity, With<TextMenu>>,
	) -> Option<Entity> {
		let mut overlay = None;
		let mut hud = None;
		let mut text = None;
		for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
			if overlays.contains(entity) {
				overlay = Some(entity);
			} else if hud_menus.contains(entity) && hud.is_none() {
				hud = Some(entity);
			} else if text_menus.contains(entity) && text.is_none() {
				text = Some(entity);
			}
		}
		Self::pick_focus(overlay, hud, text)
	}

	/// Deliver one nav event to this controller's focused [`UiTarget`].
	pub fn trigger_nav(&self, commands: &mut Commands, source: InputSource, nav: MenuNav) {
		let Some(focus) = self.focus else {
			return;
		};
		Self::trigger_routed(commands, Routed::to_ui(source, focus, nav));
	}

	pub fn trigger_routed(commands: &mut Commands, routed: Routed<MenuNav>) {
		let Some(entity) = routed.target().ui_entity() else {
			return;
		};
		commands.trigger(MenuNavImpulse::new(entity, routed.into_payload()));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn overlay_beats_hud_and_text() -> anyhow::Result<()> {
		let overlay = Entity::from_bits(1);
		let hud = Entity::from_bits(2);
		let text = Entity::from_bits(3);
		assert_eq!(MenuController::pick_focus(Some(overlay), Some(hud), Some(text)), Some(overlay));
		assert_eq!(MenuController::pick_focus(None, Some(hud), Some(text)), Some(hud));
		assert_eq!(MenuController::pick_focus(None, None, Some(text)), Some(text));
		assert_eq!(MenuController::pick_focus(None, None, None), None);
		Ok(())
	}

	#[test]
	fn focus_is_a_ui_target() {
		let mut controller = MenuController::default();
		controller.set_focus(Some(Entity::from_bits(9)));
		assert_eq!(controller.focused_entity(), Some(Entity::from_bits(9)));
		assert_eq!(controller.focus, Some(UiTarget::new(Entity::from_bits(9))));
	}
}
