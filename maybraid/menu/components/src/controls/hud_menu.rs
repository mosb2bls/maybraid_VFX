//! Shared HUD focus list.
//!
//! Pickables stamp [`HudMenuItem`]. The host inserts [`HudMenu`] on the list
//! root after painting. Keyboard and hover update `selected`; activate/focus
//! observers live with the payload type (`MenuActivate` / `MenuFocus`).

use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use crate::single_select::{KeyboardMenuNav, TextMenuInputLock};
use crate::theme::TEXT_YELLOW;
use maybraid_input::{MenuNav, MenuNavImpulse};

/// Focus index for a HUD list (panel headers or overlay leaves).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudMenu {
	pub selected: usize,
	pub item_count: usize,
}

impl HudMenu {
	pub fn new(item_count: usize) -> Self {
		Self { selected: 0, item_count }
	}

	/// Keep the previous index when a list is rebuilt.
	pub fn retain(item_count: usize, previous: Option<Self>) -> Self {
		let selected = previous
			.filter(|menu| menu.item_count > 0 && item_count > 0)
			.map(|menu| menu.selected.min(item_count - 1))
			.unwrap_or(0);
		Self { selected, item_count }
	}

	pub fn step(&mut self, delta: i32) {
		if self.item_count == 0 {
			return;
		}
		let n = self.item_count as i32;
		self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
	}

	pub fn apply_nav(&mut self, nav: MenuNav) {
		self.apply_nav_spatial(nav, &[]);
	}

	/// Arrow nav: same-row left/right and nearest-above/below when positions
	/// are known, otherwise a 1D wrap.
	pub fn apply_nav_spatial(&mut self, nav: MenuNav, items: &[(usize, Vec2)]) {
		if matches!(nav, MenuNav::Select | MenuNav::Back) {
			return;
		}
		if let Some(next) = spatial_neighbor(self.selected, nav, items) {
			self.selected = next;
			return;
		}
		match nav {
			MenuNav::Up | MenuNav::Left => self.step(-1),
			MenuNav::Down | MenuNav::Right => self.step(1),
			MenuNav::Select | MenuNav::Back => {}
		}
	}
}

/// One pickable in a [`HudMenu`]. `menu` is the list root, not necessarily
/// the item's parent (tiles nest).
#[derive(Component, Debug, Clone, Copy)]
pub struct HudMenuItem {
	pub index: usize,
	pub menu: Entity,
}

/// Overlay lists steal arrows / Enter from panel lists while they exist.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudOverlayMenu;

/// This list still receives pad nav while [`TextMenuInputLock`] is set
/// (in-game keypad while the name modal is open).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudMenuIgnoresLock;

pub fn select_hud_item_on_over(
	over: On<Pointer<Over>>,
	stack: Res<crate::modal::MenuModalStack>,
	roots: Query<(Entity, &crate::modal::MenuModal)>,
	child_of: Query<&ChildOf>,
	items: Query<&HudMenuItem>,
	mut menus: Query<&mut HudMenu>,
) {
	if !crate::modal::entity_under_nav_owner(over.entity, &stack, &roots, &child_of) {
		return;
	}
	let Ok(item) = items.get(over.entity) else {
		return;
	};
	let Ok(mut menu) = menus.get_mut(item.menu) else {
		return;
	};
	if menu.item_count == 0 {
		return;
	}
	menu.selected = item.index.min(menu.item_count - 1);
}

pub fn apply_hud_menu_nav(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	unlocked: Query<(), With<HudMenuIgnoresLock>>,
	items: Query<(&HudMenuItem, &UiGlobalTransform)>,
	mut menus: Query<&mut HudMenu>,
) {
	if lock.0 && unlocked.get(impulse.entity).is_err() {
		return;
	}
	let Ok(mut menu) = menus.get_mut(impulse.entity) else {
		return;
	};
	let positions: Vec<(usize, Vec2)> = items
		.iter()
		.filter(|(item, _)| item.menu == impulse.entity)
		.map(|(item, transform)| (item.index, transform.affine().translation))
		.collect();
	menu.apply_nav_spatial(impulse.event().nav, &positions);
}

/// Closest same-row or adjacent-row neighbor. `None` falls back to 1D step.
pub(crate) fn spatial_neighbor(
	current: usize,
	nav: MenuNav,
	items: &[(usize, Vec2)],
) -> Option<usize> {
	let origin = items.iter().find(|(index, _)| *index == current)?.1;
	let row_slop = 18.0;
	match nav {
		MenuNav::Left => items
			.iter()
			.filter(|(index, pos)| {
				*index != current && (pos.y - origin.y).abs() <= row_slop && pos.x < origin.x
			})
			.max_by(|a, b| a.1.x.partial_cmp(&b.1.x).unwrap_or(std::cmp::Ordering::Equal))
			.map(|(index, _)| *index),
		MenuNav::Right => items
			.iter()
			.filter(|(index, pos)| {
				*index != current && (pos.y - origin.y).abs() <= row_slop && pos.x > origin.x
			})
			.min_by(|a, b| a.1.x.partial_cmp(&b.1.x).unwrap_or(std::cmp::Ordering::Equal))
			.map(|(index, _)| *index),
		MenuNav::Up => items
			.iter()
			.filter(|(index, pos)| *index != current && pos.y < origin.y - 4.0)
			.min_by(|a, b| {
				let dy = (origin.y - a.1.y).partial_cmp(&(origin.y - b.1.y));
				let dx = (a.1.x - origin.x).abs().partial_cmp(&(b.1.x - origin.x).abs());
				dy.unwrap_or(std::cmp::Ordering::Equal)
					.then(dx.unwrap_or(std::cmp::Ordering::Equal))
			})
			.map(|(index, _)| *index),
		MenuNav::Down => items
			.iter()
			.filter(|(index, pos)| *index != current && pos.y > origin.y + 4.0)
			.min_by(|a, b| {
				let dy = (a.1.y - origin.y).partial_cmp(&(b.1.y - origin.y));
				let dx = (a.1.x - origin.x).abs().partial_cmp(&(b.1.x - origin.x).abs());
				dy.unwrap_or(std::cmp::Ordering::Equal)
					.then(dx.unwrap_or(std::cmp::Ordering::Equal))
			})
			.map(|(index, _)| *index),
		MenuNav::Select | MenuNav::Back => None,
	}
}

pub fn navigate_hud_menus(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	stack: Res<crate::modal::MenuModalStack>,
	roots: Query<(Entity, &crate::modal::MenuModal)>,
	child_of: Query<&ChildOf>,
	overlay_menus: Query<Entity, With<HudOverlayMenu>>,
	mut menus: Query<(Entity, &mut HudMenu)>,
) {
	if !keyboard_nav.is_enabled() || lock.0 {
		return;
	}
	let delta = if keyboard.just_pressed(KeyCode::ArrowDown)
		|| keyboard.just_pressed(KeyCode::ArrowRight)
	{
		1
	} else if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::ArrowLeft) {
		-1
	} else {
		return;
	};
	let overlay_open = !overlay_menus.is_empty();
	for (entity, mut menu) in &mut menus {
		if overlay_open != overlay_menus.contains(entity) {
			continue;
		}
		if !crate::modal::entity_under_nav_owner(entity, &stack, &roots, &child_of) {
			continue;
		}
		menu.step(delta);
	}
}

/// Yellow outline on the focused HUD pickable (tiles, swatches, keypad).
pub fn sync_hud_item_focus(
	items: Query<(Entity, &HudMenuItem)>,
	menus: Query<&HudMenu>,
	mut outlines: Query<&mut Outline>,
) {
	for (entity, item) in &items {
		let Ok(menu) = menus.get(item.menu) else {
			continue;
		};
		let Ok(mut outline) = outlines.get_mut(entity) else {
			continue;
		};
		outline.color = if item.index == menu.selected { TEXT_YELLOW } else { Color::NONE };
	}
}

#[cfg(test)]
mod tests {
	use super::HudMenu;

	#[test]
	fn step_wraps() {
		let mut menu = HudMenu::new(3);
		menu.step(-1);
		assert_eq!(menu.selected, 2);
		menu.step(1);
		assert_eq!(menu.selected, 0);
	}

	#[test]
	fn retain_clamps() {
		let previous = HudMenu { selected: 4, item_count: 5 };
		assert_eq!(HudMenu::retain(2, Some(previous)).selected, 1);
		assert_eq!(HudMenu::retain(0, Some(previous)).selected, 0);
	}

	#[test]
	fn spatial_nav_stays_on_the_row_then_steps_vertically() {
		use super::spatial_neighbor;
		use bevy::prelude::Vec2;
		use maybraid_input::MenuNav;
		let items = [
			(0, Vec2::new(0.0, 0.0)),
			(1, Vec2::new(40.0, 0.0)),
			(2, Vec2::new(80.0, 0.0)),
			(3, Vec2::new(0.0, 40.0)),
			(4, Vec2::new(40.0, 40.0)),
		];
		assert_eq!(spatial_neighbor(1, MenuNav::Left, &items), Some(0));
		assert_eq!(spatial_neighbor(1, MenuNav::Right, &items), Some(2));
		assert_eq!(spatial_neighbor(1, MenuNav::Down, &items), Some(4));
		assert_eq!(spatial_neighbor(4, MenuNav::Up, &items), Some(1));
		assert_eq!(spatial_neighbor(0, MenuNav::Left, &items), None);
	}

	#[test]
	fn keyboard_nav_skips_the_catalog_while_item_drop_owns_nav() {
		use super::{navigate_hud_menus, HudMenu, HudOverlayMenu};
		use crate::modal::{MenuModal, MenuModalId, MenuModalLayer, MenuModalStack};
		use crate::single_select::{KeyboardMenuNav, TextMenuInputLock};
		use bevy::ecs::system::RunSystemOnce;
		use bevy::prelude::*;

		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<TextMenuInputLock>();
		world.insert_resource(KeyboardMenuNav(true));
		let drop_menu = world.spawn((HudMenu::new(2), HudOverlayMenu)).id();
		let drop_root = world.spawn(MenuModal::ITEM_DROP).id();
		world.entity_mut(drop_root).add_child(drop_menu);
		let catalog = world.spawn((HudMenu::new(6), HudOverlayMenu)).id();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let mut keys = ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::ArrowDown);
		world.insert_resource(keys);
		world.run_system_once(navigate_hud_menus).expect("nav open");
		assert_eq!(world.get::<HudMenu>(catalog).expect("catalog").selected, 0);
		assert_eq!(world.get::<HudMenu>(drop_menu).expect("drop").selected, 1);
		world.resource_mut::<MenuModalStack>().close(MenuModalId::ITEM_DROP);
		let mut keys = ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::ArrowDown);
		world.insert_resource(keys);
		world.run_system_once(navigate_hud_menus).expect("nav closed");
		assert_eq!(world.get::<HudMenu>(catalog).expect("catalog").selected, 1);
	}
}
