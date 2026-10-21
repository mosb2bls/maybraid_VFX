//! UI-owned first-spawn / death-respawn picker controller.
//!
//! Confirm and highlight nav read [`MenuNavPad`], not character intents. The
//! picker claims [`CaptureKind::MenuNav`] while the locked map is open and
//! yields to a higher overlay, text entry, or another menu.

use bevy::prelude::*;
use maybraid_controllers_core::{
	CaptureKind, CaptureLayer, CaptureToken, InputRouter, InputRouting, InputSource, ScopeOwner,
};
use maybraid_input::{MenuNav, MenuNavPad};
use poi_intelligence::{PoiId, PoiRegistry};

use crate::map_view::WorldMapView;
use crate::player_lifecycle::{
	PendingPlayerRespawn, PlayerChoseRespawnPoi, WorldPlayerRespawnState,
};

/// Token this picker holds on [`InputRouter`]. Releasing it cannot release
/// another owner's capture.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnPickerCapture {
	pub token: Option<CaptureToken>,
}

/// Locked discovery-map picker. Focus does not require an assigned actor.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnPickerController {
	pub focus: bool,
}

impl SpawnPickerController {
	pub fn owner() -> ScopeOwner {
		ScopeOwner::spawn_picker()
	}

	/// Locked picker sheet is on screen and a life is waiting to be placed.
	pub fn should_own(map: &WorldMapView, pending: bool) -> bool {
		map.open && map.close_locked && pending
	}

	/// A higher overlay, text lock, or another menu owns the source.
	pub fn higher_claim_blocks(layer: CaptureLayer) -> bool {
		match layer.kind() {
			CaptureKind::TextEntry => true,
			CaptureKind::Overlay => {
				layer.owner() != ScopeOwner::map_overlay() && layer.owner() != Self::owner()
			}
			CaptureKind::MenuNav => layer.owner() != Self::owner(),
		}
	}

	pub fn accepts(router: &InputRouter) -> bool {
		router.top_capture().is_some_and(|layer| !Self::higher_claim_blocks(layer))
	}

	pub fn confirm(highlighted: Option<PoiId>) -> PlayerChoseRespawnPoi {
		PlayerChoseRespawnPoi { poi: highlighted }
	}
}

pub fn refresh_spawn_picker_focus(
	map: Res<WorldMapView>,
	state: Res<WorldPlayerRespawnState>,
	mut controller: ResMut<SpawnPickerController>,
	mut capture: ResMut<SpawnPickerCapture>,
	mut router: InputRouting,
) {
	let should_own = SpawnPickerController::should_own(&map, state.pending.is_some());
	let blocked = router.top_capture().is_some_and(|layer| {
		SpawnPickerController::higher_claim_blocks(layer)
			&& layer.owner() != SpawnPickerController::owner()
	});
	let owner = (should_own && !blocked).then_some(SpawnPickerController::owner());
	controller.focus = owner.is_some();
	router.replace_owned_capture(
		&mut capture.token,
		InputSource::local_pad(),
		owner,
		CaptureKind::MenuNav,
	);
}

pub fn dispatch_spawn_picker(
	nav: Res<MenuNavPad>,
	router: Res<InputRouter>,
	registry: Res<PoiRegistry>,
	mut controller: ResMut<SpawnPickerController>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut chosen: MessageWriter<PlayerChoseRespawnPoi>,
) {
	let Some(pending) = state.pending.as_mut() else {
		controller.focus = false;
		return;
	};
	if !controller.focus || !SpawnPickerController::accepts(&router) {
		return;
	}
	if pending.apply_menu_nav(&registry, &nav.events) {
		chosen.write(SpawnPickerController::confirm(pending.highlighted));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_controllers_core::{InputSource, UiTarget};
	use maybraid_input::{PadButton, VirtualPad, VirtualPadConfig};

	fn router_world() -> World {
		let mut world = World::new();
		world.init_resource::<WorldMapView>();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<SpawnPickerController>();
		world.init_resource::<SpawnPickerCapture>();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<maybraid_controllers_core::RoutingEvent>>();
		world.init_resource::<PoiRegistry>();
		world.init_resource::<Messages<PlayerChoseRespawnPoi>>();
		world.init_resource::<MenuNavPad>();
		world
	}

	fn lock_picker(world: &mut World) {
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		world.resource_mut::<WorldPlayerRespawnState>().pending =
			Some(PendingPlayerRespawn::test(Vec3::new(3.0, 4.0, 5.0), false));
	}

	fn press_a(world: &mut World) {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		let mut nav = MenuNavPad::default();
		nav.derive(&pad, &VirtualPadConfig::default(), 0.0);
		assert!(
			nav.just_pressed(MenuNav::Select),
			"A must become MenuNav::Select, got {:?}",
			nav.events
		);
		world.insert_resource(pad);
		world.insert_resource(nav);
	}

	fn map_capture(world: &mut World) {
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::map_overlay(),
			CaptureKind::Overlay,
		);
		assert!(world.resource::<InputRouter>().actor(InputSource::local_pad()).is_none());
		assert!(world.resource::<InputRouter>().blocks_actor(InputSource::local_pad()));
	}

	fn refresh_and_dispatch(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(refresh_spawn_picker_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(dispatch_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn chosen(world: &mut World) -> Vec<PlayerChoseRespawnPoi> {
		world.resource_mut::<Messages<PlayerChoseRespawnPoi>>().drain().collect()
	}

	#[test]
	fn a_confirms_without_an_actor_while_map_capture_is_active() -> anyhow::Result<()> {
		let mut world = router_world();
		lock_picker(&mut world);
		map_capture(&mut world);
		press_a(&mut world);
		refresh_and_dispatch(&mut world)?;
		assert_eq!(chosen(&mut world), vec![PlayerChoseRespawnPoi { poi: None }]);
		assert!(world.resource::<InputRouter>().actor(InputSource::local_pad()).is_none());
		assert!(world.resource::<InputRouter>().blocks_actor(InputSource::local_pad()));
		Ok(())
	}

	#[test]
	fn a_higher_overlay_blocks_picker_confirm() -> anyhow::Result<()> {
		let mut world = router_world();
		lock_picker(&mut world);
		map_capture(&mut world);
		world
			.run_system_once(refresh_spawn_picker_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(Entity::from_bits(11))),
			CaptureKind::Overlay,
		);
		press_a(&mut world);
		world
			.run_system_once(dispatch_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(chosen(&mut world).is_empty(), "a modal overlay above the picker must not confirm");
		Ok(())
	}

	#[test]
	fn text_capture_blocks_picker_confirm() -> anyhow::Result<()> {
		let mut world = router_world();
		lock_picker(&mut world);
		map_capture(&mut world);
		world
			.run_system_once(refresh_spawn_picker_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::console(),
			CaptureKind::TextEntry,
		);
		press_a(&mut world);
		world
			.run_system_once(dispatch_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(chosen(&mut world).is_empty(), "text capture must not confirm");
		Ok(())
	}

	#[test]
	fn another_menu_on_top_blocks_picker_confirm() -> anyhow::Result<()> {
		let mut world = router_world();
		lock_picker(&mut world);
		map_capture(&mut world);
		world
			.run_system_once(refresh_spawn_picker_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(Entity::from_bits(3))),
			CaptureKind::MenuNav,
		);
		press_a(&mut world);
		world
			.run_system_once(refresh_spawn_picker_focus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(dispatch_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(chosen(&mut world).is_empty(), "a higher menu must keep Select");
		Ok(())
	}
}
