//! `MenuNavPad` → [`MenuNavImpulse`] on the focused menu.

use bevy::prelude::*;
use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad, MenuOwnsBack};
use menu_components::{
	hud_menu_under_modal, HudMenu, HudOverlayMenu, MenuModal, MenuModalId, MenuModalStack,
	ShortTextModal, ShortTextPad, TextMenu, TextMenuInputLock,
};

use crate::controller::MenuController;

pub fn exclusive_nav_menu(
	modal: &ShortTextModal,
	stack: &MenuModalStack,
	pads: &Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	roots: &Query<(Entity, &MenuModal)>,
	menus: &Query<Entity, With<HudMenu>>,
	children: &Query<&Children>,
) -> Option<Entity> {
	if modal.is_open() || stack.contains(MenuModalId::SHORT_TEXT) {
		return pads.iter().next();
	}
	hud_menu_under_modal(stack.nav_owner()?, roots, menus, children)
}

pub fn refresh_menu_focus(
	children: Query<&Children>,
	overlays: Query<Entity, With<HudOverlayMenu>>,
	hud_menus: Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
	all_menus: Query<Entity, With<HudMenu>>,
	text_menus: Query<Entity, With<TextMenu>>,
	modal: Res<ShortTextModal>,
	stack: Res<MenuModalStack>,
	pads: Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	roots: Query<(Entity, &MenuModal)>,
	mut controllers: Query<(Entity, &mut MenuController)>,
	mut menu_owns: ResMut<MenuOwnsBack>,
) {
	let exclusive = exclusive_nav_menu(&modal, &stack, &pads, &roots, &all_menus, &children);
	let mut focused = false;
	for (root, mut controller) in &mut controllers {
		controller.focus = exclusive
			.filter(|menu| *menu == root || descendants_contain(&children, root, *menu))
			.or_else(|| {
				MenuController::resolve(root, &children, &overlays, &hud_menus, &text_menus)
			});
		focused |= controller.focus.is_some();
	}
	menu_owns.0 = focused;
}

fn descendants_contain(children: &Query<&Children>, root: Entity, target: Entity) -> bool {
	children.iter_descendants(root).any(|entity| entity == target)
}

pub fn dispatch_menu_nav(
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	stack: Res<MenuModalStack>,
	nav: Res<MenuNavPad>,
	controllers: Query<&MenuController>,
	pads: Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	roots: Query<(Entity, &MenuModal)>,
	menus: Query<Entity, With<HudMenu>>,
	children: Query<&Children>,
	mut commands: Commands,
) {
	let events: Vec<_> =
		nav.events.iter().copied().filter(|event| *event != MenuNav::Back).collect();
	if events.is_empty() {
		return;
	}
	if let Some(target) = exclusive_nav_menu(&modal, &stack, &pads, &roots, &menus, &children) {
		for event in events {
			commands.trigger(MenuNavImpulse::new(target, event));
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

#[cfg(test)]
mod tests {
	use super::{dispatch_menu_nav, exclusive_nav_menu, refresh_menu_focus};
	use crate::controller::MenuController;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad, MenuOwnsBack};
	use menu_components::{
		HudMenu, HudOverlayMenu, MenuModal, MenuModalLayer, MenuModalStack, ShortTextModal,
		ShortTextPad, TextMenuInputLock,
	};

	#[derive(Resource, Default)]
	struct NavHits(Vec<(u64, MenuNav)>);

	fn record(impulse: On<MenuNavImpulse>, menus: Query<&HudMenu>, mut hits: ResMut<NavHits>) {
		if menus.contains(impulse.entity) {
			hits.0.push((impulse.entity.to_bits(), impulse.event().nav));
		}
	}

	fn catalog_under_drop() -> (World, Entity, Entity) {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<ShortTextModal>();
		world.init_resource::<TextMenuInputLock>();
		world.init_resource::<MenuOwnsBack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<NavHits>();
		world.add_observer(record);
		let drop_menu = world.spawn((HudMenu::new(2), HudOverlayMenu)).id();
		let drop_root = world.spawn(MenuModal::ITEM_DROP).id();
		world.entity_mut(drop_root).add_child(drop_menu);
		let catalog = world.spawn((HudMenu::new(6), HudOverlayMenu)).id();
		let root = world.spawn(MenuController::default()).id();
		world.entity_mut(root).add_children(&[drop_root, catalog]);
		(world, catalog, drop_menu)
	}

	fn run_down(world: &mut World) -> anyhow::Result<Vec<(u64, MenuNav)>> {
		world.resource_mut::<MenuNavPad>().events = vec![MenuNav::Down];
		world.resource_mut::<NavHits>().0.clear();
		world
			.run_system_once(refresh_menu_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(dispatch_menu_nav)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		Ok(world.resource::<NavHits>().0.clone())
	}

	#[test]
	fn item_drop_owns_stick_nav_instead_of_the_catalog() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let hits = run_down(&mut world)?;
		assert_eq!(hits, vec![(drop_menu.to_bits(), MenuNav::Down)]);
		assert_ne!(hits[0].0, catalog.to_bits());
		Ok(())
	}

	#[test]
	fn closing_item_drop_restores_catalog_nav() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let open = run_down(&mut world)?;
		assert_eq!(open, vec![(drop_menu.to_bits(), MenuNav::Down)]);
		world
			.resource_mut::<MenuModalStack>()
			.close(menu_components::MenuModalId::ITEM_DROP);
		world.entity_mut(drop_menu).despawn();
		let closed = run_down(&mut world)?;
		assert_eq!(closed, vec![(catalog.to_bits(), MenuNav::Down)]);
		Ok(())
	}

	#[test]
	fn stick_cardinal_is_the_same_exclusive_path() -> anyhow::Result<()> {
		use maybraid_input::{MenuNav as Nav, PadButton, VirtualPad};
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::new(0.0, -0.9));
		pad.finish_digital();
		assert_eq!(Nav::from_pad(&pad, 0.5), vec![Nav::Down]);
		assert!(!pad.just_pressed(PadButton::DpadDown));
		let (mut world, _, drop_menu) = catalog_under_drop();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		world.resource_mut::<MenuNavPad>().events = Nav::from_pad(&pad, 0.5);
		world.resource_mut::<NavHits>().0.clear();
		world
			.run_system_once(refresh_menu_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(dispatch_menu_nav)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert_eq!(world.resource::<NavHits>().0, vec![(drop_menu.to_bits(), Nav::Down)]);
		Ok(())
	}

	fn exclusive_target(
		modal: Res<ShortTextModal>,
		stack: Res<MenuModalStack>,
		pads: Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
		roots: Query<(Entity, &MenuModal)>,
		menus: Query<Entity, With<HudMenu>>,
		children: Query<&Children>,
	) -> Option<Entity> {
		exclusive_nav_menu(&modal, &stack, &pads, &roots, &menus, &children)
	}

	#[test]
	fn exclusive_nav_menu_is_none_for_overlay_only() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		world.entity_mut(drop_menu).despawn();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		let target = world
			.run_system_once(exclusive_target)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(target, None);
		let hits = run_down(&mut world)?;
		assert_eq!(hits, vec![(catalog.to_bits(), MenuNav::Down)]);
		Ok(())
	}

	#[test]
	fn short_text_still_wins_over_item_drop() -> anyhow::Result<()> {
		let (mut world, _, _) = catalog_under_drop();
		let pad = world.spawn((ShortTextPad, HudMenu::new(8))).id();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		let target = world
			.run_system_once(exclusive_target)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(target, Some(pad));
		Ok(())
	}
}
