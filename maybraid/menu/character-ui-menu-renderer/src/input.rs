//! Pointer and keyboard → [`MenuFocus`] / [`MenuActivate`] / overlay events.

use bevy::prelude::*;
use maybraid_input::{MenuNav, MenuNavImpulse};
use menu_components::{
	HudMenu, HudMenuItem, HudOverlayMenu, KeyboardMenuNav, MenuActivate, MenuFocus,
	MenuModalDismiss, MenuModalId, TextMenuInputLock,
};

use crate::event::{OverlayClose, OverlayOpen};
use crate::widgets::{CloseOverlaySelect, MenuButton, OpenSelectKey, OverlaySelectRoot};

pub fn emit_hud_activate_on_click<E: Copy + Send + Sync + 'static>(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	buttons: Query<&MenuButton<E>>,
	mut commands: Commands,
) {
	if lock.0 {
		return;
	}
	let Ok(button) = buttons.get(click.entity) else {
		return;
	};
	commands.trigger(MenuActivate { entity: click.entity, choice: button.0 });
}

pub fn emit_overlay_open_on_click(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	keys: Query<&OpenSelectKey>,
	mut commands: Commands,
) {
	if lock.0 {
		return;
	}
	let Ok(key) = keys.get(click.entity) else {
		return;
	};
	commands.trigger(OverlayOpen { entity: click.entity, key: key.0 });
}

pub fn emit_overlay_close_on_click(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	closes: Query<(), With<CloseOverlaySelect>>,
	mut commands: Commands,
) {
	if lock.0 || closes.get(click.entity).is_err() {
		return;
	}
	commands.trigger(OverlayClose { entity: click.entity });
}

pub fn emit_hud_focus<E: Copy + Send + Sync + 'static>(
	menus: Query<(Entity, &HudMenu), Changed<HudMenu>>,
	items: Query<(&HudMenuItem, &MenuButton<E>)>,
	mut commands: Commands,
) {
	for (menu_entity, menu) in &menus {
		if let Some((_, button)) = items
			.iter()
			.find(|(item, _)| item.menu == menu_entity && item.index == menu.selected)
		{
			commands.trigger(MenuFocus { entity: menu_entity, choice: button.0 });
		}
	}
}

pub fn emit_hud_activate_on_enter<E: Copy + Send + Sync + 'static>(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<(Entity, &HudMenu), With<HudOverlayMenu>>,
	panel_menus: Query<(Entity, &HudMenu), Without<HudOverlayMenu>>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled() || lock.0 || !keyboard.just_pressed(KeyCode::Enter) {
		return;
	}
	let Some((menu_entity, menu)) =
		overlay_menus.iter().next().or_else(|| panel_menus.iter().next())
	else {
		return;
	};
	activate_selected_hud_item(menu_entity, menu, &items, &mut commands);
}

pub fn emit_hud_activate_on_nav<E: Copy + Send + Sync + 'static>(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	menus: Query<&HudMenu>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	mut commands: Commands,
) {
	if lock.0 || impulse.event().nav != MenuNav::Select {
		return;
	}
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	activate_selected_hud_item(impulse.entity, menu, &items, &mut commands);
}

fn activate_selected_hud_item<E: Copy + Send + Sync + 'static>(
	menu_entity: Entity,
	menu: &HudMenu,
	items: &Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	commands: &mut Commands,
) {
	for (entity, item, button, open, close) in items.iter() {
		if item.menu != menu_entity || item.index != menu.selected {
			continue;
		}
		if let Some(button) = button {
			commands.trigger(MenuActivate { entity, choice: button.0 });
		}
		if let Some(open) = open {
			commands.trigger(OverlayOpen { entity, key: open.0 });
		}
		if close.is_some() {
			commands.trigger(OverlayClose { entity });
		}
		break;
	}
}

pub fn close_overlay_on_modal_dismiss(
	mut dismiss: MessageReader<MenuModalDismiss>,
	roots: Query<Entity, With<OverlaySelectRoot>>,
	mut commands: Commands,
) {
	if !dismiss.read().any(|event| event.is(MenuModalId::OVERLAY_SELECT)) {
		return;
	}
	let Ok(root) = roots.single() else {
		return;
	};
	commands.trigger(OverlayClose { entity: root });
}

#[cfg(test)]
mod tests {
	use super::{close_overlay_on_modal_dismiss, OverlaySelectRoot};
	use crate::event::OverlayClose as OverlayCloseEvent;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_input::{MenuNav, MenuNavPad};
	use menu_components::{
		consume_menu_modal_back, consume_screen_back, MenuModalDismiss, MenuModalId,
		MenuModalLayer, MenuModalStack, ScreenBackPressed,
	};

	#[derive(Resource, Default)]
	struct OverlayCloseHits(u32);

	fn count_close(_close: On<OverlayCloseEvent>, mut hits: ResMut<OverlayCloseHits>) {
		hits.0 += 1;
	}

	fn overlay_back_world() -> World {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<OverlayCloseHits>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.init_resource::<Messages<ScreenBackPressed>>();
		world.add_observer(count_close);
		world.spawn(OverlaySelectRoot);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_overlay_on_modal_dismiss).expect("close");
		world.flush();
		world
	}

	#[test]
	fn back_closes_overlay_and_does_not_leave_the_screen() {
		let mut world = overlay_back_world();
		assert_eq!(world.resource::<OverlayCloseHits>().0, 1);
		assert_eq!(
			world
				.run_system_once(|mut dismiss: MessageReader<MenuModalDismiss>| {
					dismiss.read().next().map(|event| event.id)
				})
				.expect("dismiss"),
			Some(MenuModalId::OVERLAY_SELECT)
		);
		let left = world
			.run_system_once(
				|nav: Res<MenuNavPad>,
				 stack: Res<MenuModalStack>,
				 mut backs: MessageReader<ScreenBackPressed>| {
					consume_screen_back(&nav, &stack, &mut backs)
				},
			)
			.expect("screen back");
		assert!(!left);
	}

	#[test]
	fn escape_closes_overlay_without_nav_back() {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<OverlayCloseHits>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.add_observer(count_close);
		world.spawn(OverlaySelectRoot);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_overlay_on_modal_dismiss).expect("close");
		world.flush();
		assert_eq!(world.resource::<OverlayCloseHits>().0, 1);
	}
}
