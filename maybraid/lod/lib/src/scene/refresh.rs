//! LOD refresh + sync driven by region / level messages.
//!
//! Submodules:
//! - [`regions`] — strategy `P` + nodes `F` → [`LodSceneRefreshRegion<M>`]
//! - [`cull_regions`] — rotating cull lattice → untyped AABB cache → erased enqueue
//! - [`levels`] — per-domain region AABBs → shared viewer snapshots + host-hit cache → one erased level pass
//! - [`entities`] — one untyped fold: max level → write [`crate::LodSceneLevel`]
//! - [`sync`] — root sync, chunk fulfill, cull
//!
//! Plugins:
//! - [`LodRefreshCorePlugin`] — sets, node track, untyped level fold, root sync (once)
//! - [`LodSceneRefreshRegionPlugin<P, F, M>`] — region production
//! - [`LodSceneCullRegionPlugin<P, F, M>`] — cull region production
//! - [`LodSceneRefreshLevelsFillPlugin<I>`] — once per index: all [`crate::LodNode`] snapshots + host hits
//! - [`LodSceneRefreshLevelsPlugin<T>`] — register `T` with the shared emitter
//! - [`LodSceneRefreshSyncPlugin<T, F>`] — chunk fulfill + optional full-scan cull
//! - [`LodSceneCullProduceFillPlugin<I>`] — once per index: all [`crate::LodNode`] snapshots + host hits
//! - [`LodSceneRegionCullPlugin<I, M, T, F>`] — producer registration over shared enqueue
//! - [`LodSceneRefreshPlugin<T, M, I, F>`] — fill + emit + sync (region separate)

mod bounds;
pub mod cull_regions;
pub mod entities;
pub mod levels;
pub mod regions;
pub mod sync;
mod viewer;

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::lod_ref::{LodNodePlugin, LodNodeSystems};
use crate::scene::host::{
	drain_lod_tree_vis, settle_lod_level_root_visibility, sync_lod_level_roots, LodTreeVisBudget,
	LodTreeVisQueue,
};
use crate::scene::region_index::LodSceneHostIndex;
use crate::scene::SemanticLodScene;

pub use bounds::LodHostBounds;
pub use cull_regions::{
	fill_lod_cull_produce_cache, produce_lod_cull_for_region, produce_lod_cull_for_region_erased,
	produce_lod_cull_regions, sync_cullable_roots_marker, sync_nested_refresh_allowed,
	LodCullMarkerPlugin, LodCullProduceCache, LodCullProduceCadence, LodCullRegionCursor,
	LodCullRegions, LodCullRegionsStatus, LodHostHasCullableRoots, LodNestedRefreshAllowed,
	LodNestedRefreshBlocked, LodNestedRefreshSyncBudget, LodSceneCullAabb,
	LodSceneCullProduceFillPlugin, LodSceneCullRegion, LodSceneCullRegionPlugin,
	LodSceneRegionCullPlugin, OpenLattice,
};
pub use entities::{
	dominant_lod_ref, refresh_lod_host_levels, update_lod_host_levels,
	LodSceneRefreshEntitiesPlugin,
};
pub use levels::{
	fill_lod_produce_cache, produce_lod_refresh_levels, produce_lod_refresh_levels_erased,
	LodLevelProducer, LodProduceCache, LodRefreshDomain, LodRefreshMembership, LodSceneRefreshAabb,
	LodSceneRefreshLevel, LodSceneRefreshLevelsFillPlugin, LodSceneRefreshLevelsPlugin,
};
pub use regions::{
	produce_lod_refresh_regions, Bullseye, LodRefreshRegions, LodRefreshRegionsError,
	LodRefreshRegionsStatus, LodSceneRefreshRegion, LodSceneRefreshRegionPlugin, Spotlight,
};
pub use sync::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, add_lod_refresh_cull_for,
	apply_lod_cull_requests, begin_chunk_lod_fulfill, begin_chunk_lod_fulfill_erased,
	cancel_unstarted_cull_for_desired_pending_roots,
	cancel_unstarted_cull_for_desired_pending_roots_erased, complete_chunk_lod_fulfill,
	cull_lod_level_roots, drain_chunk_lod_fulfill, drain_lod_cull, enqueue_lod_cull,
	reset_lod_chunk_budget, LodChunkAtomicOverrun, LodChunkBeginScanCursor, LodChunkBudgetClock,
	LodChunkBudgetPlugin, LodChunkCullSystems, LodChunkDrainDiagnostics, LodChunkFulfillBudget,
	LodChunkFulfillSystems, LodChunkFulfillment, LodCullInFlight, LodCullRequest, LodLazyPending,
	LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed, LodSceneRefreshChunkPlugin,
	LodSceneRefreshSyncPlugin,
};
pub use viewer::LodViewer;

/// System set ordering for refresh + sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodRefreshSystems {
	/// Advance [`crate::lod_ref::LodNodePose`] from [`Transform`].
	Track,
	/// Produce [`LodSceneRefreshRegion`] messages.
	ProduceRegions,
	/// Produce [`LodSceneRefreshLevel`] messages from regions + index.
	ProduceLevels,
	/// Write desired [`crate::LodSceneLevel`] on hosts.
	UpdateLevels,
	/// Show/hide roots and enqueue [`crate::LodLevelSpawnRequest`].
	SyncRoots,
	/// Spawn missing level-root content (chunk fulfill).
	Fulfill,
	/// Enqueue + budgeted teardown of inactive level roots.
	Cull,
	/// Budgeted descendant [`Disabled`] apply for hide/show trees.
	///
	/// [`crate::LodTreeVisBudget::time_per_frame`] is a per-visit target, not a
	/// hard cap — one wide [`Children`] copy can overrun.
	VisDrain,
}

/// Order inside [`LodRefreshSystems::ProduceLevels`]: fill the shared cache, then emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodLevelProduceSystems {
	/// Every [`crate::LodNode`] snapshot once, then host hits per produce domain ([`fill_lod_produce_cache`]).
	FillCache,
	/// Shared hit-driven level emission.
	Emit,
}

pub(crate) fn configure_refresh_sets(app: &mut App) {
	app.configure_sets(
		Update,
		(
			LodRefreshSystems::Track.after(LodNodeSystems::Track),
			LodRefreshSystems::ProduceRegions,
			LodRefreshSystems::ProduceLevels,
			LodRefreshSystems::UpdateLevels,
			LodRefreshSystems::SyncRoots,
			LodRefreshSystems::Fulfill,
			LodRefreshSystems::Cull,
			LodRefreshSystems::VisDrain,
		)
			.chain(),
	);
	app.configure_sets(
		Update,
		(LodLevelProduceSystems::FillCache, LodLevelProduceSystems::Emit)
			.chain()
			.in_set(LodRefreshSystems::ProduceLevels),
	);
}

pub(crate) fn ensure_refresh_core(app: &mut App) {
	if !app.is_plugin_added::<LodRefreshCorePlugin>() {
		app.add_plugins(LodRefreshCorePlugin);
	}
}

/// Untyped refresh infrastructure: sets, node tracking, level fold, root sync.
pub struct LodRefreshCorePlugin;

impl Plugin for LodRefreshCorePlugin {
	fn build(&self, app: &mut App) {
		configure_refresh_sets(app);
		if !app.is_plugin_added::<LodNodePlugin>() {
			app.add_plugins(LodNodePlugin);
		}
		crate::jobs::ensure_lod_job_counter(app);
		app.init_resource::<LodProduceCache>()
			.init_resource::<LodCullProduceCache>()
			.init_resource::<LodTreeVisBudget>()
			.init_resource::<LodTreeVisQueue>()
			.add_message::<LodSceneRefreshAabb>()
			.add_message::<LodSceneCullAabb>()
			.add_message::<LodSceneRefreshLevel>()
			.add_systems(
				Update,
				(
					refresh_lod_host_levels.in_set(LodRefreshSystems::UpdateLevels),
					produce_lod_refresh_levels_erased.in_set(LodLevelProduceSystems::Emit),
					settle_lod_level_root_visibility
						.before(sync_lod_level_roots)
						.in_set(LodRefreshSystems::SyncRoots),
					sync_lod_level_roots.in_set(LodRefreshSystems::SyncRoots),
					drain_lod_tree_vis.in_set(LodRefreshSystems::VisDrain),
				),
			);
	}
}

/// Fill + shared typed-callback emit + chunk sync. `I` is an untyped
/// [`LodSceneHostIndex`].
///
/// Fill is once per `I` ([`LodSceneRefreshLevelsFillPlugin`]) and snapshots every
/// [`crate::LodNode`]. `F` still selects sync / full-scan cull drivers, not fill.
/// Channel `M` stamps [`LodRefreshMembership`] on host `T` so produce only
/// emits for matching [`LodSceneRefreshAabb`] domains. Channels that share `M`
/// (bullseye + spotlight) union into one query. Add
/// [`LodSceneRefreshRegionPlugin`] separately for region production.
/// Use [`Self::without_full_scan_cull`] with [`LodSceneRegionCullPlugin`] for
/// lattice-scoped cull enqueue.
pub struct LodSceneRefreshPlugin<T, M, I, F = With<LodViewer>>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	full_scan_cull: bool,
	_marker: PhantomData<fn() -> (T, M, I, F)>,
}

impl<T, M, I, F> Default for LodSceneRefreshPlugin<T, M, I, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { full_scan_cull: true, _marker: PhantomData }
	}
}

impl<T, M, I, F> LodSceneRefreshPlugin<T, M, I, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	pub fn without_full_scan_cull() -> Self {
		Self { full_scan_cull: false, _marker: PhantomData }
	}
}

impl<T, M, I, F> Plugin for LodSceneRefreshPlugin<T, M, I, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		if !app.is_plugin_added::<LodSceneRefreshLevelsFillPlugin<I>>() {
			app.add_plugins(LodSceneRefreshLevelsFillPlugin::<I>::default());
		}
		if !app.is_plugin_added::<LodSceneRefreshLevelsPlugin<T>>() {
			app.add_plugins(LodSceneRefreshLevelsPlugin::<T>::default());
		}
		app.add_observer(levels::attach_refresh_membership::<T, M>);
		if !app.is_plugin_added::<LodSceneRefreshSyncPlugin<T, F>>() {
			if self.full_scan_cull {
				app.add_plugins(LodSceneRefreshSyncPlugin::<T, F>::default());
			} else {
				app.add_plugins(LodSceneRefreshSyncPlugin::<T, F>::without_full_scan_cull());
			}
		}
	}
}

/// Compatibility alias for [`LodSceneRefreshRegionPlugin`].
pub type LodRefreshProductionPlugin<P, F, M> = LodSceneRefreshRegionPlugin<P, F, M>;

#[cfg(test)]
mod tests;
