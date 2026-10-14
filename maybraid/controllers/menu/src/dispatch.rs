//! `MenuNavPad` → [`MenuNavImpulse`] on the focused menu.

use bevy::prelude::*;
use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad, MenuOwnsBack};
use menu_components::{
	HudMenu, HudOverlayMenu, MenuModalId, MenuModalStack, ShortTextModal, ShortTextPad, TextMenu,
	TextMenuInputLock,
};

use crate::controller::MenuController;

pub fn refresh_menu_focus(
	children: Query<&Children>,
	overlays: Query<Entity, With<HudOverlayMenu>>,
	hud_menus: Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
	text_menus: Query<Entity, With<TextMenu>>,
	mut controllers: Query<(Entity, &mut MenuController)>,
	mut menu_owns: ResMut<MenuOwnsBack>,
) {
	let mut focused = false;
	for (root, mut controller) in &mut controllers {
		controller.focus =
			MenuController::resolve(root, &children, &overlays, &hud_menus, &text_menus);
		focused |= controller.focus.is_some();
	}
	menu_owns.0 = focused;
}

pub fn dispatch_menu_nav(
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	stack: Res<MenuModalStack>,
	nav: Res<MenuNavPad>,
	controllers: Query<&MenuController>,
	pads: Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	mut commands: Commands,
) {
	let events: Vec<_> =
		nav.events.iter().copied().filter(|event| *event != MenuNav::Back).collect();
	if events.is_empty() {
		return;
	}
	if modal.is_open() || stack.contains(MenuModalId::SHORT_TEXT) {
		let Some(pad) = pads.iter().next() else {
			return;
		};
		for event in events {
			commands.trigger(MenuNavImpulse::new(pad, event));
		}
		return;
	}
	if modal.dismissed || stack.consumed() {
		return;
	}
	if lock.0 {
		return;
	}
	for controller in &controllers {
		let Some(focus) = controller.focus else {
			continue;
		};
		for event in &events {
			commands.trigger(MenuNavImpulse::new(focus, *event));
		}
	}
}
