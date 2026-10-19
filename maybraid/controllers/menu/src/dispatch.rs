//! `MenuNavPad` → [`MenuNavImpulse`] on the focused menu.

use bevy::prelude::*;
use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad, MenuOwnsBack};
use menu_components::{
	hud_menu_under_modal, HudMenu, HudOverlayMenu, MenuModal, MenuModalId, MenuModalStack,
	ShortTextModal, ShortTextPad, TextMenu, TextMenuInputLock,
};

use crate::controller::MenuController;

/// One modal owns directional nav, or none does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusiveNav {
	/// Overlay catalog (or no modal). Parent controllers may handle stick / keys.
	Unclaimed,
	/// A nav-owning layer is on top, but its HudMenu has not spawned yet.
	/// Dispatch must not fall through to the parent catalog.
	Awaiting,
	/// Route stick / D-pad / keyboard to this menu.
	Target(Entity),
}

/// Derive the exclusive nav target from the stack's top owning layer.
pub fn exclusive_nav_menu(
	_modal: &ShortTextModal,
	stack: &MenuModalStack,
	pads: &Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	roots: &Query<(Entity, &MenuModal)>,
	menus: &Query<Entity, With<HudMenu>>,
	children: &Query<&Children>,
) -> ExclusiveNav {
	let Some(id) = stack.nav_owner() else {
		return ExclusiveNav::Unclaimed;
	};
	let target = match id {
		MenuModalId::SHORT_TEXT => pads.iter().next(),
		_ => hud_menu_under_modal(id, roots, menus, children),
	};
	match target {
		Some(entity) => ExclusiveNav::Target(entity),
		None => ExclusiveNav::Awaiting,
	}
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
		controller.focus = match exclusive {
			ExclusiveNav::Target(menu)
				if menu == root || descendants_contain(&children, root, menu) =>
			{
				Some(menu)
			}
			ExclusiveNav::Target(_) | ExclusiveNav::Awaiting => None,
			ExclusiveNav::Unclaimed => {
				MenuController::resolve(root, &children, &overlays, &hud_menus, &text_menus)
			}
		};
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
	match exclusive_nav_menu(&modal, &stack, &pads, &roots, &menus, &children) {
		ExclusiveNav::Target(target) => {
			for event in events {
				commands.trigger(MenuNavImpulse::new(target, event));
			}
			return;
		}
		ExclusiveNav::Awaiting => return,
		ExclusiveNav::Unclaimed => {}
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
	use super::{dispatch_menu_nav, exclusive_nav_menu, refresh_menu_focus, ExclusiveNav};
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
	) -> ExclusiveNav {
		exclusive_nav_menu(&modal, &stack, &pads, &roots, &menus, &children)
	}

	#[test]
	fn exclusive_nav_menu_is_unclaimed_for_overlay_only() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		world.entity_mut(drop_menu).despawn();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		let target = world
			.run_system_once(exclusive_target)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(target, ExclusiveNav::Unclaimed);
		let hits = run_down(&mut world)?;
		assert_eq!(hits, vec![(catalog.to_bits(), MenuNav::Down)]);
		Ok(())
	}

	#[test]
	fn top_layer_owns_nav_instead_of_any_open_short_text() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		let pad = world.spawn((ShortTextPad, HudMenu::new(8))).id();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let target = world
			.run_system_once(exclusive_target)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(target, ExclusiveNav::Target(drop_menu));
		assert_ne!(target, ExclusiveNav::Target(pad));
		let hits = run_down(&mut world)?;
		assert_eq!(hits, vec![(drop_menu.to_bits(), MenuNav::Down)]);
		assert_ne!(hits[0].0, catalog.to_bits());
		Ok(())
	}

	#[test]
	fn awaiting_owner_does_not_fall_through_to_the_catalog() -> anyhow::Result<()> {
		let (mut world, _catalog, drop_menu) = catalog_under_drop();
		world.entity_mut(drop_menu).despawn();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let target = world
			.run_system_once(exclusive_target)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(target, ExclusiveNav::Awaiting);
		let hits = run_down(&mut world)?;
		assert!(
			hits.is_empty(),
			"catalog must stay isolated while the owner is awaiting, got {hits:?}"
		);
		Ok(())
	}
}
