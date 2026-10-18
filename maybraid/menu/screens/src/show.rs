//! Shared spawn path for full-screen menu roots.
//!
//! Screens swap in one place: [`MenuScreenSystems::Show`] in `PostUpdate`.
//! `Update` logic only queues `RequestShow*` entities, so no `Update` system
//! can write into a screen that another system despawned in the same command
//! flush. When several requests land in one frame the newest wins and the
//! rest are dropped, so two screens never spawn together.

use std::sync::atomic::{AtomicU64, Ordering};

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::ecs::system::{ScheduleSystem, SystemParam};
use bevy::prelude::*;
use bevy::ui::UiSystems;

use crate::MenuScreen;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuScreenSystems {
	/// Apply the newest `RequestShow*`: despawn every screen and request, spawn one.
	Show,
}

/// Required by every `RequestShow*`. Orders requests queued in the same frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MenuShowRequest(u64);

impl MenuShowRequest {
	pub fn next() -> Self {
		static NEXT: AtomicU64 = AtomicU64::new(0);
		Self(NEXT.fetch_add(1, Ordering::Relaxed))
	}
}

/// Live menu screens plus queued show requests.
#[derive(SystemParam)]
pub struct MenuScreens<'w, 's> {
	screens: Query<'w, 's, Entity, With<MenuScreen>>,
	requests: Query<'w, 's, (Entity, &'static MenuShowRequest)>,
}

impl MenuScreens<'_, '_> {
	/// Despawn every screen and every pending show request.
	pub fn clear(&self, commands: &mut Commands) {
		for entity in self.screens.iter().chain(self.requests.iter().map(|(entity, _)| entity)) {
			commands.entity(entity).try_despawn();
		}
	}

	pub fn newest_request(&self) -> Option<Entity> {
		self.requests.iter().max_by_key(|(_, order)| **order).map(|(entity, _)| entity)
	}

	/// If the newest pending request matches `requests`, clear every screen and
	/// request and return it. The caller then spawns its screen.
	pub fn take<D: QueryData, F: QueryFilter>(
		&self,
		commands: &mut Commands,
		requests: &Query<D, F>,
	) -> Option<Entity> {
		let newest = self.newest_request()?;
		if !requests.contains(newest) {
			return None;
		}
		self.clear(commands);
		Some(newest)
	}
}

/// Register screen-swap systems in [`MenuScreenSystems::Show`].
pub fn add_menu_show_systems<M>(
	app: &mut App,
	systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
) {
	app.configure_sets(
		PostUpdate,
		MenuScreenSystems::Show
			.before(UiSystems::Prepare)
			.before(TransformSystems::Propagate),
	)
	.add_systems(PostUpdate, systems.in_set(MenuScreenSystems::Show));
}

#[cfg(test)]
mod tests {
	use super::{add_menu_show_systems, MenuScreens, MenuShowRequest};
	use crate::MenuScreen;
	use bevy::prelude::*;

	#[derive(Component)]
	#[require(MenuShowRequest = MenuShowRequest::next())]
	struct RequestA;

	#[derive(Component)]
	#[require(MenuShowRequest = MenuShowRequest::next())]
	struct RequestB;

	#[derive(Component)]
	struct ScreenA;

	#[derive(Component)]
	struct ScreenB;

	fn show_a(mut commands: Commands, screens: MenuScreens, requests: Query<(), With<RequestA>>) {
		if screens.take(&mut commands, &requests).is_some() {
			commands.spawn((ScreenA, MenuScreen));
		}
	}

	fn show_b(mut commands: Commands, screens: MenuScreens, requests: Query<(), With<RequestB>>) {
		if screens.take(&mut commands, &requests).is_some() {
			commands.spawn((ScreenB, MenuScreen));
		}
	}

	fn count<T: Component>(app: &mut App) -> usize {
		let world = app.world_mut();
		world.query_filtered::<(), With<T>>().iter(world).count()
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		add_menu_show_systems(&mut app, (show_a, show_b));
		app
	}

	#[test]
	fn the_newest_request_wins_and_the_rest_drop() {
		let mut app = app();
		app.world_mut().spawn(RequestA);
		app.world_mut().spawn(RequestB);
		app.update();
		assert_eq!(count::<ScreenA>(&mut app), 0);
		assert_eq!(count::<ScreenB>(&mut app), 1);
		assert_eq!(count::<MenuShowRequest>(&mut app), 0);

		app.world_mut().spawn(RequestB);
		app.world_mut().spawn(RequestA);
		app.update();
		assert_eq!(count::<ScreenA>(&mut app), 1);
		assert_eq!(count::<ScreenB>(&mut app), 0);
		assert_eq!(count::<MenuScreen>(&mut app), 1);
	}

	#[test]
	fn clear_drops_screens_and_pending_requests() {
		let mut app = app();
		app.world_mut().spawn((ScreenA, MenuScreen));
		app.world_mut().spawn(RequestB);
		app.world_mut()
			.run_system_cached(|mut commands: Commands, screens: MenuScreens| {
				screens.clear(&mut commands);
			})
			.expect("clear");
		app.update();
		assert_eq!(count::<MenuScreen>(&mut app), 0);
		assert_eq!(count::<MenuShowRequest>(&mut app), 0);
	}
}
