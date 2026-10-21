//! `MenuNavPad` → [`MenuNavImpulse`] on the focused [`UiTarget`].

use bevy::prelude::*;
use maybraid_controllers_core::{
	CaptureKind, CaptureToken, InputRouter, InputRouting, InputSource, Routed, ScopeOwner, UiTarget,
};
use maybraid_input::{MenuNav, MenuNavPad};
use menu_components::{
	hud_menu_under_modal, HudMenu, HudOverlayMenu, MenuModal, MenuModalId, MenuModalStack,
	ShortTextModal, ShortTextPad, TextMenu, TextMenuInputLock,
};

use crate::controller::MenuController;

/// Tokens this menu controller holds on [`InputRouter`]. Releasing one cannot
/// release another owner's capture.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuNavCapture {
	pub token: Option<CaptureToken>,
	pub awaiting: Option<CaptureToken>,
	pub text: Option<CaptureToken>,
}

/// One modal owns directional nav, or none does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusiveNav {
	/// Overlay catalog (or no modal). Parent controllers may handle stick / keys.
	Unclaimed,
	/// A nav-owning layer is on top, but its HudMenu has not spawned yet.
	/// Dispatch must not fall through to the parent catalog.
	Awaiting,
	/// Route stick / D-pad / keyboard to this menu.
	Target(UiTarget),
}

impl ExclusiveNav {
	pub fn target(self) -> Option<UiTarget> {
		match self {
			Self::Target(target) => Some(target),
			Self::Unclaimed | Self::Awaiting => None,
		}
	}

	pub fn target_entity(self) -> Option<Entity> {
		self.target().map(UiTarget::entity)
	}

	/// Derive the exclusive nav target from the stack's top owning layer.
	pub fn resolve(
		_modal: &ShortTextModal,
		stack: &MenuModalStack,
		pads: &Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
		roots: &Query<(Entity, &MenuModal)>,
		menus: &Query<Entity, With<HudMenu>>,
		children: &Query<&Children>,
	) -> Self {
		let Some(id) = stack.nav_owner() else {
			return Self::Unclaimed;
		};
		let target = match id {
			MenuModalId::SHORT_TEXT => pads.iter().next(),
			_ => hud_menu_under_modal(id, roots, menus, children),
		};
		match target {
			Some(entity) => Self::Target(UiTarget::new(entity)),
			None => Self::Awaiting,
		}
	}

	pub fn capture_owner(self) -> Option<ScopeOwner> {
		self.target().map(ScopeOwner::ui)
	}
}

/// Derive the exclusive nav target from the stack's top owning layer.
pub fn exclusive_nav_menu(
	modal: &ShortTextModal,
	stack: &MenuModalStack,
	pads: &Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	roots: &Query<(Entity, &MenuModal)>,
	menus: &Query<Entity, With<HudMenu>>,
	children: &Query<&Children>,
) -> ExclusiveNav {
	ExclusiveNav::resolve(modal, stack, pads, roots, menus, children)
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
	mut router: InputRouting,
	mut menu_capture: ResMut<MenuNavCapture>,
) {
	let exclusive = ExclusiveNav::resolve(&modal, &stack, &pads, &roots, &all_menus, &children);
	let mut owner = exclusive.capture_owner();
	for (root, mut controller) in &mut controllers {
		controller.focus = match exclusive {
			ExclusiveNav::Target(menu) if menu.is_under(root, &children) => Some(menu),
			ExclusiveNav::Target(_) | ExclusiveNav::Awaiting => None,
			ExclusiveNav::Unclaimed => {
				MenuController::resolve(root, &children, &overlays, &hud_menus, &text_menus)
					.map(UiTarget::new)
			}
		};
		if owner.is_none() {
			if let Some(focus) = controller.focus {
				owner = Some(ScopeOwner::ui(focus));
			}
		}
	}
	router.replace_owned_capture(
		&mut menu_capture.token,
		InputSource::local_pad(),
		owner,
		CaptureKind::MenuNav,
	);
	router.replace_owned_capture(
		&mut menu_capture.awaiting,
		InputSource::local_pad(),
		matches!(exclusive, ExclusiveNav::Awaiting).then_some(ScopeOwner::awaiting_nav()),
		CaptureKind::Overlay,
	);
}

pub fn dispatch_menu_nav(
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	stack: Res<MenuModalStack>,
	nav: Res<MenuNavPad>,
	router: Res<InputRouter>,
	controllers: Query<&MenuController>,
	mut commands: Commands,
) {
	let events: Vec<_> =
		nav.events.iter().copied().filter(|event| *event != MenuNav::Back).collect();
	if events.is_empty() {
		return;
	}
	if let Some(layer) = router.top_capture() {
		if layer.kind() == CaptureKind::MenuNav {
			if let Some(target) = layer.ui_owner() {
				for event in events {
					MenuController::trigger_routed(
						&mut commands,
						Routed::to_ui(layer.source(), target, event),
					);
				}
			}
			return;
		}
		if layer.kind().precedes_menu_nav() {
			return;
		}
	}
	if modal.dismissed || stack.consumed() {
		return;
	}
	if lock.0 {
		return;
	}
	for controller in &controllers {
		for event in &events {
			controller.trigger_nav(&mut commands, InputSource::local_pad(), *event);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{dispatch_menu_nav, exclusive_nav_menu, refresh_menu_focus, ExclusiveNav};
	use crate::controller::MenuController;
	use crate::dispatch::MenuNavCapture;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_controllers_core::{CaptureKind, InputRouter, InputSource, ScopeOwner, UiTarget};
	use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad};
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
		world.init_resource::<MenuNavPad>();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<maybraid_controllers_core::RoutingEvent>>();
		world.init_resource::<MenuNavCapture>();
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
	fn exclusive_capture_is_the_dispatch_target() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let hits = run_down(&mut world)?;
		assert_eq!(hits, vec![(drop_menu.to_bits(), MenuNav::Down)]);
		let owner = world
			.resource::<InputRouter>()
			.top_capture_of_kind(CaptureKind::MenuNav)
			.ok_or_else(|| anyhow::anyhow!("menu capture"))?;
		assert_eq!(owner.ui_owner(), Some(UiTarget::new(drop_menu)));
		assert_ne!(owner.ui_owner(), Some(UiTarget::new(catalog)));
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

	#[test]
	fn exclusive_nav_is_unclaimed_for_overlay_only() -> anyhow::Result<()> {
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
		assert_eq!(target, ExclusiveNav::Target(UiTarget::new(drop_menu)));
		assert_ne!(target, ExclusiveNav::Target(UiTarget::new(pad)));
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
		assert_eq!(
			world.resource::<InputRouter>().top_capture().map(|layer| layer.kind()),
			Some(CaptureKind::Overlay)
		);
		assert!(world.resource::<InputRouter>().blocks_actor(InputSource::local_pad()));
		Ok(())
	}

	#[test]
	fn higher_overlay_blocks_an_existing_menu() -> anyhow::Result<()> {
		let (mut world, catalog, drop_menu) = catalog_under_drop();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let open = run_down(&mut world)?;
		assert_eq!(open, vec![(drop_menu.to_bits(), MenuNav::Down)]);
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::map_overlay(),
			CaptureKind::Overlay,
		);
		world.resource_mut::<MenuNavPad>().events = vec![MenuNav::Down];
		world.resource_mut::<NavHits>().0.clear();
		world
			.run_system_once(dispatch_menu_nav)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert!(
			world.resource::<NavHits>().0.is_empty(),
			"overlay above menu must block catalog and drop nav, got {:?}",
			world.resource::<NavHits>().0
		);
		assert_ne!(catalog.to_bits(), drop_menu.to_bits());
		Ok(())
	}

	#[test]
	fn plugin_wired_overlay_stays_above_a_refreshed_menu() -> anyhow::Result<()> {
		use crate::MenuControllerPlugin;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			bevy::asset::AssetPlugin::default(),
			MenuControllerPlugin,
		));
		app.add_message::<bevy::input::keyboard::KeyboardInput>()
			.add_message::<bevy::window::Ime>()
			.add_message::<bevy::input::mouse::MouseMotion>()
			.add_message::<bevy::input::gamepad::GamepadRumbleRequest>()
			.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<ButtonInput<MouseButton>>()
			.init_resource::<MenuNavPad>()
			.init_resource::<bevy::picking::hover::HoverMap>()
			.init_resource::<NavHits>()
			.add_observer(record);
		let catalog = app.world_mut().spawn((HudMenu::new(4), HudOverlayMenu)).id();
		let root = app.world_mut().spawn(MenuController::default()).id();
		app.world_mut().entity_mut(root).add_child(catalog);
		app.world_mut().resource_mut::<MenuNavPad>().events = vec![MenuNav::Down];
		app.finish();
		app.cleanup();
		app.world_mut().run_schedule(Update);
		app.world_mut().flush();
		assert_eq!(app.world().resource::<NavHits>().0, vec![(catalog.to_bits(), MenuNav::Down)]);

		app.world_mut().resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::map_overlay(),
			CaptureKind::Overlay,
		);
		app.world_mut().resource_mut::<NavHits>().0.clear();
		app.world_mut().resource_mut::<MenuNavPad>().events = vec![MenuNav::Down];
		app.world_mut().run_schedule(Update);
		app.world_mut().flush();
		assert!(
			app.world().resource::<NavHits>().0.is_empty(),
			"refresh must keep the overlay on top of the existing menu"
		);
		assert_eq!(
			app.world().resource::<InputRouter>().top_capture().map(|layer| layer.kind()),
			Some(CaptureKind::Overlay)
		);
		Ok(())
	}

	#[test]
	fn dispatch_without_refresh_does_not_use_an_exclusive_fallback() -> anyhow::Result<()> {
		let (mut world, _catalog, drop_menu) = catalog_under_drop();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		world.resource_mut::<MenuNavPad>().events = vec![MenuNav::Down];
		world
			.run_system_once(dispatch_menu_nav)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		anyhow::ensure!(
			world.resource::<NavHits>().0.is_empty(),
			"dispatch must wait for refresh to write capture, not re-resolve ExclusiveNav"
		);
		assert!(world
			.resource::<InputRouter>()
			.top_capture_of_kind(CaptureKind::MenuNav)
			.is_none());
		assert_ne!(drop_menu, Entity::PLACEHOLDER);
		Ok(())
	}
}
