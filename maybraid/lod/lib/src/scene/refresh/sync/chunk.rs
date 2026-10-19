//! Incremental LOD level-root fulfillment via [`crate::SceneChunk`].
//!
//! Default sync path for level-root fulfillment:
//! builds a pending root, drains weighted primitives under
//! [`LodChunkFulfillBudget`], marks content [`LodLevelRootStreamed`], then
//! completes when next-level nested [`LodSceneHost`]s are [`LodSceneHostStreamed`].
//!
//! Scheduling (begin + drain):
//! - **Presence** (~¼): cold jobs (empty → something).
//! - **Desired** (~⅜): warm jobs for the host's desired level root.
//! - **Active** (~⅜): warm jobs on a shown (non-Hidden) non-desired root — warm-hold.
//! Drain ranks `(parent_desired, self_level)` High→… within each class.
//! Begin admits by count ([`LodChunkFulfillBudget::begins_per_frame`]) and shared
//! begin weight ([`LodChunkFulfillBudget::begin_weights_per_frame`], sum of
//! primitive weights). Classified candidates are sorted by viewer XZ distance
//! within each class / near-far list. The shared candidate scan is capped
//! ([`LodChunkFulfillBudget::begin_scan_per_frame`]) and skipped when the clock
//! is empty. Active begin quota folds into Desired.
//! Complete caps visibility swaps ([`LodChunkFulfillBudget::completes_per_frame`]).
//! Cull drain uses a shallow nested-host scan and recursive-despawns ready roots.
//! Frame parity rotates class order; leftovers cascade.
//!
//! Pipeline (within [`crate::LodRefreshSystems::Fulfill`]):
//! reset budget → cancel unstarted cull on desired pending → begin →
//! drain → complete.

mod begin;
mod complete;
mod drain;
mod resume;
mod schedule;
mod types;
mod util;

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::scene::refresh::LodSceneRefreshLevelsPlugin;
use crate::scene::SemanticLodScene;

use super::super::cull_regions::produce_lod_cull_for_region_erased;
use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodRefreshSystems};
use super::cull::{apply_lod_cull_requests, cull_lod_level_roots, drain_lod_cull, LodCullRequest};

pub use begin::begin_chunk_lod_fulfill_erased;
use begin::LodChunkBeginCandidateScratch;
pub use complete::{bump_nested_streamed_progress, complete_chunk_lod_fulfill};
pub use drain::drain_chunk_lod_fulfill;
pub use resume::cancel_unstarted_cull_for_desired_pending_roots_erased;
pub use schedule::reset_lod_chunk_budget;
pub use types::{
	FulfillClass, LodChunkAtomicOverrun, LodChunkBeginClock, LodChunkBeginScanCursor,
	LodChunkBudgetClock, LodChunkDrainCursor, LodChunkDrainDiagnostics, LodChunkFulfillBudget,
	LodChunkFulfillment, LodCullInFlight, LodLazyPending, LodLevelRootPending,
	LodLevelRootStreamed, LodSceneHostStreamed,
};

/// Register incremental chunk fulfill systems for one [`SemanticLodScene`] host type.
pub fn add_lod_refresh_chunk_for<T: Component + SemanticLodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<T>>() {
		app.add_plugins(LodSceneRefreshChunkPlugin::<T>::default());
	}
}

/// Chunk fulfill plugin (default sync path for level-root spawn).
pub struct LodSceneRefreshChunkPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for LodSceneRefreshChunkPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

/// Substeps within [`LodRefreshSystems::Cull`] (order against these, not system types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodChunkCullSystems {
	/// Snapshots + untyped host hits ([`crate::fill_lod_cull_produce_cache`]).
	FillCache,
	/// Mark unwanted roots / hosts ([`cull_lod_level_roots`] / region cull).
	Enqueue,
	/// Apply [`LodCullRequest`] → [`LodCullInFlight`].
	Apply,
	/// Budgeted leaf-first despawn.
	Drain,
}

/// Substeps within [`LodRefreshSystems::Fulfill`] after budget reset.
///
/// Begin, drain, and complete are each registered once and dispatch semantic
/// scene work through the host's type-erased producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodChunkFulfillSystems {
	/// Cancel unstarted [`LodCullInFlight`] on desired pending roots.
	Resume,
	/// Shared [`begin_chunk_lod_fulfill_erased`].
	Begin,
	/// Shared exclusive semantic spawn drain (`World::spawn_scene` + time budget).
	Drain,
	/// Shared warm-swap complete.
	Complete,
}

/// Shared chunk budget clock, cull messages, and one-shot drain/complete registration.
pub struct LodChunkBudgetPlugin;

impl Plugin for LodChunkBudgetPlugin {
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<LodChunkFulfillBudget>()
			.init_resource::<LodChunkBudgetClock>()
			.init_resource::<LodChunkBeginClock>()
			.init_resource::<LodChunkBeginScanCursor>()
			.init_resource::<LodChunkBeginCandidateScratch>()
			.init_resource::<LodChunkDrainCursor>()
			.init_resource::<LodChunkDrainDiagnostics>()
			.add_message::<LodCullRequest>()
			.configure_sets(
				Update,
				(
					LodChunkCullSystems::FillCache,
					LodChunkCullSystems::Enqueue,
					LodChunkCullSystems::Apply,
					LodChunkCullSystems::Drain,
				)
					.chain()
					.in_set(LodRefreshSystems::Cull),
			)
			.configure_sets(
				Update,
				(
					LodChunkFulfillSystems::Resume,
					LodChunkFulfillSystems::Begin,
					LodChunkFulfillSystems::Drain,
					LodChunkFulfillSystems::Complete,
				)
					.chain()
					.in_set(LodRefreshSystems::Fulfill)
					.after(reset_lod_chunk_budget),
			)
			.add_systems(
				Update,
				(
					reset_lod_chunk_budget.in_set(LodRefreshSystems::Fulfill),
					cancel_unstarted_cull_for_desired_pending_roots_erased
						.in_set(LodChunkFulfillSystems::Resume),
					begin_chunk_lod_fulfill_erased.in_set(LodChunkFulfillSystems::Begin),
					drain_chunk_lod_fulfill.in_set(LodChunkFulfillSystems::Drain),
					bump_nested_streamed_progress
						.in_set(LodChunkFulfillSystems::Complete)
						.before(complete_chunk_lod_fulfill),
					complete_chunk_lod_fulfill.in_set(LodChunkFulfillSystems::Complete),
					produce_lod_cull_for_region_erased.in_set(LodChunkCullSystems::Enqueue),
					apply_lod_cull_requests.in_set(LodChunkCullSystems::Apply),
					drain_lod_cull.in_set(LodChunkCullSystems::Drain),
				),
			);
	}
}

fn ensure_chunk_budget(app: &mut App) {
	if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
		app.add_plugins(LodChunkBudgetPlugin);
	}
}

impl<T> Plugin for LodSceneRefreshChunkPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_chunk_budget(app);
		if !app.is_plugin_added::<LodSceneRefreshLevelsPlugin<T>>() {
			app.add_plugins(LodSceneRefreshLevelsPlugin::<T>::default());
		}
	}
}

/// Probe-style levels + chunk fulfill + cull (no region message pipeline).
pub fn add_lod_refresh_chunk_full_for<T: Component + SemanticLodScene>(app: &mut App) {
	ensure_chunk_budget(app);
	app.add_systems(
		Update,
		(
			crate::scene::refresh::update_lod_host_levels::<T, (), With<LodViewer>>
				.in_set(LodRefreshSystems::UpdateLevels),
			cull_lod_level_roots::<T, (), With<LodViewer>>.in_set(LodChunkCullSystems::Enqueue),
		),
	);
	add_lod_refresh_chunk_for::<T>(app);
}

/// Default sync for message-driven refresh: chunk fulfill + optional full-scan cull.
///
/// Prefer [`Self::without_full_scan_cull`] when using
/// [`crate::scene::refresh::cull_regions::LodSceneRegionCullPlugin`].
pub struct LodSceneRefreshSyncPlugin<T, F = With<LodViewer>>
where
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
{
	full_scan_cull: bool,
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { full_scan_cull: true, _marker: PhantomData }
	}
}

impl<T, F> LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
{
	/// Chunk fulfill only — region cull plugins own enqueue.
	pub fn without_full_scan_cull() -> Self {
		Self { full_scan_cull: false, _marker: PhantomData }
	}
}

impl<T, F> Plugin for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_chunk_budget(app);
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<T>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<T>::default());
		}
		if self.full_scan_cull {
			app.add_systems(
				Update,
				cull_lod_level_roots::<T, (), F>.in_set(LodChunkCullSystems::Enqueue),
			);
		}
	}
}
