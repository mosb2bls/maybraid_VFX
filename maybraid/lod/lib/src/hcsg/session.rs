//! HCSG session restart: advance the epoch, clear every derived store, reseed
//! installed roots.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Res, ResMut, Resource, SystemSet, Update};

use super::demand::HcsgDemand;
use super::runtime::ensure_runtime;
use super::storage::HcsgStorage;

/// Runs after [`HcsgSessionBegin`] and before [`HcsgSessionEnd`].
/// Each installed layer adds its root seeding here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct HcsgSessionSeed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
struct HcsgSessionBegin;

/// Runs after [`HcsgSessionSeed`]; use to order follow-up work after reseeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct HcsgSessionRestarted;

/// Set `true` to restart on the next [`HcsgSessionBegin`] in `Update`.
#[derive(Resource, Default)]
pub struct HcsgRestartRequest(bool);

impl HcsgRestartRequest {
	/// Queue a restart for the next `Update` pass.
	pub fn request(&mut self) {
		self.0 = true;
	}

	/// Resource value that restarts on the first `Update`.
	pub fn queued() -> Self {
		Self(true)
	}
}

#[derive(Resource, Default)]
struct HcsgSessionReseeding(bool);

#[derive(Resource)]
struct HcsgSessionInstalled;

fn reseeding_active(reseeding: Res<HcsgSessionReseeding>) -> bool {
	reseeding.0
}

/// Advances the epoch, clears derived storage, and runs every [`HcsgSessionSeed`]
/// system when [`HcsgRestartRequest`] is set.
fn begin_hcsg_session_restart(
	mut pending: ResMut<HcsgRestartRequest>,
	mut reseeding: ResMut<HcsgSessionReseeding>,
	storage: Res<HcsgStorage>,
	demand: Res<HcsgDemand>,
) {
	if !pending.0 {
		return;
	}
	pending.0 = false;
	demand.advance_epoch();
	storage.clear_derived();
	reseeding.0 = true;
}

fn end_hcsg_session_restart(mut reseeding: ResMut<HcsgSessionReseeding>) {
	reseeding.0 = false;
}

/// Queue a restart for the next `Update` pass.
pub fn request_hcsg_session_restart(mut pending: ResMut<HcsgRestartRequest>) {
	pending.request();
}

/// Ensures the full session restart chain finishes before [`HcsgSystems`].
pub(crate) fn configure_session_before_hcsg_systems(app: &mut App) {
	use super::runtime::HcsgSystems;
	app.configure_sets(Update, HcsgSessionRestarted.before(HcsgSystems));
}

/// Installs restart resources and the `Update` restart chain.
pub(crate) fn ensure_session(app: &mut App) {
	if app.world().contains_resource::<HcsgSessionInstalled>() {
		return;
	}
	app.init_resource::<HcsgRestartRequest>()
		.init_resource::<HcsgSessionReseeding>()
		.insert_resource(HcsgSessionInstalled)
		.configure_sets(Update, (HcsgSessionBegin, HcsgSessionSeed, HcsgSessionRestarted).chain())
		.add_systems(Update, begin_hcsg_session_restart.in_set(HcsgSessionBegin))
		.add_systems(Update, end_hcsg_session_restart.in_set(HcsgSessionRestarted));
}

/// Registers `system` to seed roots during a restart. Only runs while the
/// session is reseeding.
pub fn register_session_seed<M, S>(app: &mut App, system: S)
where
	S: bevy::ecs::system::IntoSystem<(), (), M> + 'static,
{
	ensure_runtime(app);
	ensure_session(app);
	app.add_systems(Update, system.run_if(reseeding_active).in_set(HcsgSessionSeed));
}

pub struct HcsgSessionPlugin;

impl Plugin for HcsgSessionPlugin {
	fn build(&self, app: &mut App) {
		ensure_session(app);
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use bevy::prelude::{App, MinimalPlugins, Update};

	use super::*;
	use crate::gen::{Id, OriginalId};
	use crate::hcsg::{GenerationContext, GenerationScheme};
	use crate::hcsg::worker::HcsgWorker;

	#[derive(Clone, Copy)]
	struct SessionRoot {
		seed: u32,
	}

	#[derive(Clone, Copy, PartialEq, Eq, Debug)]
	struct OrphanDerived {
		seed: u32,
	}

	impl GenerationScheme for SessionRoot {
		fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
			vec![OriginalId::universal()]
		}

		fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
			None
		}
	}

	impl GenerationScheme for OrphanDerived {
		fn original_ids_for(_: &mut GenerationContext, _region: Aabb3d) -> Vec<OriginalId> {
			vec![OriginalId::universal()]
		}

		fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
			let root = cx.get::<SessionRoot>(Id::Universal)?;
			let bounds = id.origin_cell_bounds().unwrap_or_else(region);
			Some((Self { seed: root.seed }, bounds))
		}
	}

	fn region() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)
	}

	fn seed_roots(storage: &HcsgStorage, seed: u32) {
		storage.seed(SessionRoot { seed }, region());
	}

	fn seed_orphan(storage: &HcsgStorage, seed: u32) {
		storage.publish(Id::Universal, Arc::new(OrphanDerived { seed }), region());
	}

	fn seed_durham_like(roots: Res<SessionRootHolder>, storage: Res<HcsgStorage>) {
		storage.seed(SessionRoot { seed: roots.seed }, region());
	}

	#[derive(Resource, Clone, Copy)]
	struct SessionRootHolder {
		seed: u32,
	}

	#[test]
	fn clear_derived_drops_unlisted_types_and_reseed_restores_roots() {
		let storage = HcsgStorage::default();
		seed_roots(&storage, 1);
		seed_orphan(&storage, 1);
		assert!(storage.get::<OrphanDerived>(Id::Universal).is_some());
		storage.clear_derived();
		assert_eq!(storage.rebuilds_after_eviction(), (0, 0));
		assert!(storage.get::<OrphanDerived>(Id::Universal).is_none());
		assert!(storage.get::<SessionRoot>(Id::Universal).is_none());
		seed_roots(&storage, 2);
		assert_eq!(storage.get::<SessionRoot>(Id::Universal).map(|r| r.seed), Some(2));
	}

	#[test]
	fn restart_runs_only_registered_seeders() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_session_seed(&mut app, seed_durham_like);
		app.insert_resource(SessionRootHolder { seed: 9 });
		let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
		seed_orphan(&storage, 3);
		app.world_mut().insert_resource(storage);
		let demand = app.world().resource::<HcsgDemand>().clone();
		let _worker =
			HcsgWorker::spawn(app.world().resource::<HcsgStorage>().clone(), demand.clone())?;
		app.world_mut().resource_mut::<HcsgRestartRequest>().request();
		app.update();
		let storage = app.world().resource::<HcsgStorage>();
		assert!(storage.get::<OrphanDerived>(Id::Universal).is_none());
		assert_eq!(storage.get::<SessionRoot>(Id::Universal).map(|r| r.seed), Some(9));
		Ok(())
	}

	#[test]
	fn derived_values_do_not_leak_across_restart() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_session_seed(&mut app, seed_durham_like);
		app.insert_resource(SessionRootHolder { seed: 1 });
		let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
		seed_roots(&storage, 1);
		let mut cx = GenerationContext::new(&storage);
		let first = cx
			.get_or_generate::<OrphanDerived>(Id::Universal)
			.ok_or_else(|| anyhow::anyhow!("first derived"))?;
		assert_eq!(first.seed, 1);
		app.world_mut().insert_resource(storage);
		app.world_mut().insert_resource(SessionRootHolder { seed: 2 });
		app.world_mut().resource_mut::<HcsgRestartRequest>().request();
		app.update();
		let storage = app.world().resource::<HcsgStorage>();
		assert_eq!(storage.get::<SessionRoot>(Id::Universal).map(|r| r.seed), Some(2));
		let mut cx = GenerationContext::new(storage);
		let rebuilt = cx
			.get_or_generate::<OrphanDerived>(Id::Universal)
			.ok_or_else(|| anyhow::anyhow!("derived after restart"))?;
		assert_eq!(rebuilt.seed, 2, "stale derived value must not survive restart");
		Ok(())
	}

	#[derive(Resource, Default)]
	struct HcsgSystemsObservedDuringReseed(bool);

	fn mark_hcsg_systems_if_reseeding(
		reseeding: Res<HcsgSessionReseeding>,
		mut observed: ResMut<HcsgSystemsObservedDuringReseed>,
	) {
		if reseeding.0 {
			observed.0 = true;
		}
	}

	#[test]
	fn restart_finishes_reseeding_before_hcsg_systems() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init_resource::<HcsgSystemsObservedDuringReseed>();
		register_session_seed(&mut app, seed_durham_like);
		app.insert_resource(SessionRootHolder { seed: 1 });
		app.add_systems(Update, mark_hcsg_systems_if_reseeding.in_set(crate::hcsg::HcsgSystems));
		app.world_mut().resource_mut::<HcsgRestartRequest>().request();
		app.update();
		anyhow::ensure!(
			!app.world().resource::<HcsgSystemsObservedDuringReseed>().0,
			"generation and presentation must not run until reseeding finishes"
		);
		Ok(())
	}
}
