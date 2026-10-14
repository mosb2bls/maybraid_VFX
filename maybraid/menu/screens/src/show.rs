//! Shared spawn path for full-screen menu roots.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Marker on every queued full-screen show request so recovery can drop them
/// together with the live [`crate::MenuScreen`] roots.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuShowRequest;

/// Live menu roots plus any queued show requests.
#[derive(SystemParam)]
pub struct MenuScreens<'w, 's> {
	screens: Query<'w, 's, Entity, With<crate::MenuScreen>>,
	requests: Query<'w, 's, Entity, With<MenuShowRequest>>,
}

impl MenuScreens<'_, '_> {
	/// Tear down every full-screen menu and every pending show request.
	///
	/// Same path normal navigation uses: [`despawn_menu_screens`] with
	/// `try_despawn`. Does not spawn a replacement.
	pub fn clear(&self, commands: &mut Commands) {
		despawn_menu_screens(commands, self.screens.iter());
		despawn_menu_screens(commands, self.requests.iter());
	}

	pub fn screen_count(&self) -> usize {
		self.screens.iter().count()
	}

	pub fn request_count(&self) -> usize {
		self.requests.iter().count()
	}
}

/// Spawn a show-request entity that [`MenuScreens::clear`] can find.
pub fn spawn_menu_show_request(commands: &mut Commands, bundle: impl Bundle) {
	commands.spawn((MenuShowRequest, bundle));
}

/// Despawn full-screen menu roots (home, in-game, character, loading).
pub fn despawn_menu_screens(commands: &mut Commands, existing: impl IntoIterator<Item = Entity>) {
	for entity in existing {
		commands.entity(entity).try_despawn();
	}
}

/// If any request entities exist, despawn every [`crate::MenuScreen`] and those
/// requests. Returns whether the caller should spawn a replacement.
pub fn take_menu_show_request(
	commands: &mut Commands,
	request_entities: impl IntoIterator<Item = Entity>,
	existing: impl IntoIterator<Item = Entity>,
) -> bool {
	let requests: Vec<Entity> = request_entities.into_iter().collect();
	if requests.is_empty() {
		return false;
	}
	despawn_menu_screens(commands, existing);
	despawn_menu_screens(commands, requests);
	true
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;

	use super::{spawn_menu_show_request, MenuScreens, MenuShowRequest};
	use crate::MenuScreen;

	#[derive(Component)]
	struct TestShowRequest;

	fn clear_menus(mut commands: Commands, screens: MenuScreens) {
		screens.clear(&mut commands);
	}

	#[test]
	fn clear_drops_screens_and_pending_requests() -> anyhow::Result<()> {
		let mut world = World::new();
		world.spawn(MenuScreen);
		let mut commands = world.commands();
		spawn_menu_show_request(&mut commands, TestShowRequest);
		world.flush();
		anyhow::ensure!(world.query::<&MenuScreen>().iter(&world).count() == 1);
		anyhow::ensure!(world.query::<&MenuShowRequest>().iter(&world).count() == 1);

		world
			.run_system_once(clear_menus)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();

		anyhow::ensure!(world.query::<&MenuScreen>().iter(&world).count() == 0);
		anyhow::ensure!(world.query::<&MenuShowRequest>().iter(&world).count() == 0);
		anyhow::ensure!(world.query::<&TestShowRequest>().iter(&world).count() == 0);
		Ok(())
	}
}
