//! Shared GameController lifecycle: teardown schedule and routing/native pairing.
//!
//! The Apple backend keeps native `GCController` handles in thread-local storage
//! and sendable routing in ECS. Those two maps must stay paired. Teardown must
//! run only on `AppExit` — `Last` itself runs every frame, and draining native
//! pads there is what crashed macOS startup in v0.1.4 when a pad was attached
//! ([#1262](https://github.com/ramate-io/maybraid/pull/1262)).

use bevy::prelude::*;

/// How to reconcile ECS routing with thread-local native state for one pad id.
///
/// Referenced from the Apple backend; Linux `cargo check` only sees the tests.
#[cfg_attr(not(any(test, target_os = "macos", target_os = "ios")), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PadMapAction {
	/// Both maps have the id — poll the existing pad.
	Existing,
	/// Routing is missing — treat the pad as newly connected.
	ConnectNew,
	/// Routing is present but native was dropped — rebuild native only.
	RecreateNative,
}

/// Decide how to pair routing vs native for one controller id. Never panics.
#[cfg_attr(not(any(test, target_os = "macos", target_os = "ios")), allow(dead_code))]
pub(crate) fn pad_map_action(has_routing: bool, has_native: bool) -> PadMapAction {
	match (has_routing, has_native) {
		(true, true) => PadMapAction::Existing,
		(false, _) => PadMapAction::ConnectNew,
		(true, false) => PadMapAction::RecreateNative,
	}
}

/// Look up a paired routing/native entry. Missing either side is `None`.
#[cfg(test)]
pub(crate) fn paired_mut<'a, R, N>(
	routing: &'a mut std::collections::HashMap<usize, R>,
	native: &'a mut std::collections::HashMap<usize, N>,
	id: usize,
) -> Option<(&'a mut R, &'a mut N)> {
	let routing = routing.get_mut(&id)?;
	let native = native.get_mut(&id)?;
	Some((routing, native))
}

/// Register native teardown in `Last` **only** when `AppExit` is posted.
///
/// `Last` runs every frame. The v0.1.4 crash scheduled `shutdown` on `Last`
/// with no run condition, so frame 1 drained `NATIVE_PADS` while routing still
/// listed the pad and frame 2 `expect`ed a native entry that was gone.
#[cfg_attr(not(any(test, target_os = "macos", target_os = "ios")), allow(dead_code))]
pub(crate) fn add_gamecontroller_shutdown<M>(app: &mut App, shutdown: impl IntoSystem<(), (), M>) {
	app.add_systems(Last, shutdown.run_if(on_message::<AppExit>));
}

#[cfg(test)]
mod tests {
	use std::collections::{HashMap, HashSet};

	use bevy::prelude::*;

	use super::{add_gamecontroller_shutdown, pad_map_action, paired_mut, PadMapAction};

	#[derive(Resource, Default)]
	struct FakeNativePads {
		ids: HashSet<usize>,
	}

	#[derive(Resource, Default)]
	struct FakeRouting {
		ids: HashSet<usize>,
	}

	fn drain_native_pads(mut pads: ResMut<FakeNativePads>) {
		pads.ids.clear();
	}

	fn sync_fake_controller(mut routing: ResMut<FakeRouting>, mut native: ResMut<FakeNativePads>) {
		const ID: usize = 1;
		match pad_map_action(routing.ids.contains(&ID), native.ids.contains(&ID)) {
			PadMapAction::ConnectNew => {
				routing.ids.insert(ID);
				native.ids.insert(ID);
			}
			PadMapAction::RecreateNative => {
				native.ids.insert(ID);
			}
			PadMapAction::Existing => {}
		}
	}

	fn app_with_gated_shutdown() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<FakeNativePads>()
			.init_resource::<FakeRouting>();
		add_gamecontroller_shutdown(&mut app, drain_native_pads);
		app
	}

	#[test]
	fn pairing_covers_all_map_combinations() -> anyhow::Result<()> {
		assert_eq!(pad_map_action(true, true), PadMapAction::Existing);
		assert_eq!(pad_map_action(false, false), PadMapAction::ConnectNew);
		assert_eq!(pad_map_action(false, true), PadMapAction::ConnectNew);
		assert_eq!(pad_map_action(true, false), PadMapAction::RecreateNative);
		Ok(())
	}

	#[test]
	fn paired_lookup_is_none_when_either_map_is_missing() -> anyhow::Result<()> {
		let mut routing = HashMap::from([(1usize, "route")]);
		let mut native: HashMap<usize, &str> = HashMap::new();
		assert!(paired_mut(&mut routing, &mut native, 1).is_none());

		native.insert(1, "native");
		routing.remove(&1);
		assert!(paired_mut(&mut routing, &mut native, 1).is_none());

		routing.insert(1, "route");
		assert!(paired_mut(&mut routing, &mut native, 1).is_some());
		Ok(())
	}

	#[test]
	fn native_state_survives_multiple_updates_without_app_exit() -> anyhow::Result<()> {
		let mut app = app_with_gated_shutdown();
		app.world_mut().resource_mut::<FakeNativePads>().ids.insert(1);
		app.world_mut().resource_mut::<FakeRouting>().ids.insert(1);

		app.update();
		app.update();
		app.update();

		anyhow::ensure!(
			app.world().resource::<FakeNativePads>().ids.contains(&1),
			"gated Last teardown must not drain native pads on a normal frame"
		);
		Ok(())
	}

	#[test]
	fn missing_native_is_recreated_instead_of_panic() -> anyhow::Result<()> {
		let mut app = app_with_gated_shutdown();
		app.add_systems(PreUpdate, sync_fake_controller);
		app.world_mut().resource_mut::<FakeRouting>().ids.insert(1);

		app.update();
		app.update();

		anyhow::ensure!(
			app.world().resource::<FakeNativePads>().ids.contains(&1),
			"routing-without-native must rebuild native, not panic"
		);
		anyhow::ensure!(
			app.world().resource::<FakeRouting>().ids.contains(&1),
			"repair must keep the existing routing id"
		);
		Ok(())
	}

	#[test]
	fn missing_routing_is_treated_as_new() -> anyhow::Result<()> {
		let mut app = app_with_gated_shutdown();
		app.add_systems(PreUpdate, sync_fake_controller);
		app.world_mut().resource_mut::<FakeNativePads>().ids.insert(1);

		app.update();

		anyhow::ensure!(
			app.world().resource::<FakeRouting>().ids.contains(&1),
			"native-without-routing must connect as new"
		);
		anyhow::ensure!(
			app.world().resource::<FakeNativePads>().ids.contains(&1),
			"connect-as-new must keep a native entry"
		);
		Ok(())
	}

	#[test]
	fn shutdown_drains_native_only_on_app_exit() -> anyhow::Result<()> {
		let mut app = app_with_gated_shutdown();
		app.world_mut().resource_mut::<FakeNativePads>().ids.insert(1);
		app.update();
		anyhow::ensure!(
			app.world().resource::<FakeNativePads>().ids.contains(&1),
			"teardown must stay idle until AppExit"
		);

		app.world_mut().write_message(AppExit::Success);
		app.update();
		anyhow::ensure!(
			app.world().resource::<FakeNativePads>().ids.is_empty(),
			"AppExit should run native teardown"
		);
		Ok(())
	}
}
