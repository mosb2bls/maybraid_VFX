//! Markers, fulfill plans, and per-frame budgets.

use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::scene::level::LodSceneLevel;

/// Marker: this [`crate::LodLevelRoot`] is still awaiting warm-swap completion.
///
/// Content may already be [`LodLevelRootStreamed`] while nested hosts catch up.
/// Cold-start roots may be visible while pending.
#[derive(Debug, Clone, Copy, Default, Component)]
#[component(on_add = crate::jobs::count_lod_job_add, on_remove = crate::jobs::count_lod_job_remove)]
pub struct LodLevelRootPending;

/// This level root's chunk plan is fully spawned (full scene representation).
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootStreamed;

/// Descendant still waiting for instance mesh + material (kit SceneRef / merge).
///
/// Stamped on the loading entity by vegetation BSN helpers. A removers outside
/// `lod` pops it when self or a descendant has `Mesh3d` and
/// `MaterialRefApplied`. [`super::complete::complete_chunk_lod_fulfill`] will
/// not warm-swap while any descendant still has this marker.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLazyPending;

/// This [`crate::LodSceneHost`] has a full scene representation available (Streamed).
///
/// Means at least one level root finished content streaming and its next-level
/// nested hosts were Streamed. Does **not** require the host to be at its
/// current desired [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHostStreamed;

/// Entity is in budgeted teardown under [`super::super::cull::drain_lod_cull`].
///
/// Inserted when a [`super::super::cull::LodCullRequest`] is applied — **not** when a
/// fulfill job is merely not desired (those stay pending with their queue paused).
/// Once [`Self::started`], the fulfill plan is dropped and cannot resume.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodCullInFlight {
	/// True after the first teardown step (plan cleared / child despawned).
	pub started: bool,
}

/// Remaining weighted primitives for a pending level root.
///
/// Frozen at begin: host mutability does not rewrite this queue mid-job.
/// May still contain [`crate::SceneChunk::Lazy`] / [`crate::SceneChunk::SubChunks`]
/// until drain / begin prefill expands them.
#[derive(Component)]
pub struct LodChunkFulfillment {
	pub queue: VecDeque<crate::scene::chunk::SceneChunk>,
	/// Primitive count at job begin (content-Streamed when `spawned == expected`).
	pub expected: usize,
	pub spawned: usize,
	/// No present sibling root when the job began (cold / presence work).
	///
	/// "Present" includes pending roots — only a truly empty host is cold.
	pub cold: bool,
	/// Owning [`crate::LodSceneHost`] (`root → LodLevelRoots → host`), frozen at begin.
	pub host: Entity,
	/// Nearest ancestor host desired at begin ([`LodSceneLevel::High`] if top-level).
	/// Used for drain `(parent, self)` ranking only.
	pub parent_desired: LodSceneLevel,
	/// Nested [`LodSceneHost`]s under this root that are [`LodSceneHostStreamed`].
	pub nested_streamed: usize,
	/// Nested host count required for warm-swap; set once when content completes.
	/// `None` until the first content-complete observation.
	pub nested_required: Option<usize>,
}

impl LodChunkFulfillment {
	pub(super) fn is_content_complete(&self) -> bool {
		self.queue.is_empty() && self.spawned >= self.expected
	}

	pub(super) fn nested_ready(&self) -> bool {
		match self.nested_required {
			Some(required) => self.nested_streamed >= required,
			None => false,
		}
	}
}

/// Per-frame weight / begin budgets for spawn vs cull (independent clocks).
#[derive(Resource, Debug, Clone, Copy)]
pub struct LodChunkFulfillBudget {
	/// Relative weight units for drain each frame.
	pub spawn_weights_per_frame: u32,
	/// Wall-clock budget for exclusive semantic [`super::drain::drain_chunk_lod_fulfill`].
	///
	/// Combined with [`Self::spawn_weights_per_frame`] as `min(weight_share, elapsed)`.
	/// Drain stops **before** the next `pull_primitive` when this is exhausted.
	pub spawn_time_per_frame: Duration,
	/// Warn (and record on [`LodChunkDrainDiagnostics`]) if one pull+spawn
	/// quantum exceeds this. Default 3 ms.
	pub max_atomic_spawn_cost: Duration,
	/// Relative weight units for cull drain each frame.
	///
	/// Ready roots charge [`LodChunkFulfillment::spawned`] (or child count) against
	/// this when the whole entity is despawned in one command.
	pub cull_weights_per_frame: u32,
	/// Max ready roots/hosts that may recursive-despawn in one frame even when
	/// their spawned weight exceeds [`Self::cull_weights_per_frame`].
	///
	/// Also the per-`Last` cap for HCSG [`crate::hcsg::RetiredHost`] teardown
	/// ([`Self::retired_host_despawns_per_frame`]).
	pub cull_root_despawns_per_frame: u32,
	/// Max new fulfill jobs started per frame (shared across all host `T`).
	pub begins_per_frame: u32,
	/// Max hosts with [`crate::LodLevelSpawnRequest`] the shared begin system may
	/// classify this frame (round-robin scan). Admission still uses
	/// [`Self::begins_per_frame`] after sorting classified candidates by
	/// viewer XZ distance. Empty clocks skip the scan entirely.
	pub begin_scan_per_frame: u32,
	/// Relative weight charged when starting fulfill jobs (sum of **prefilled**
	/// primitive weights). Lazy tails are charged later by drain.
	///
	/// Caps how much [`crate::SceneChunk`] work begin may materialize per frame,
	/// independent of [`Self::begins_per_frame`] count admission.
	pub begin_weights_per_frame: u32,
	/// Max weight begin may materialize into primitives for a single new job
	/// (rest stays [`crate::SceneChunk::Lazy`] for drain).
	pub begin_prefill_weights_per_job: u32,
	/// Max warm-swaps (`Visibility::Inherited` on the ready root, `Hidden` on siblings)
	/// per frame.
	///
	/// Content-[`LodLevelRootStreamed`] and nested-host bookkeeping still run for
	/// every ready job; only the visibility swap is capped so a completion wave
	/// does not reveal hundreds of already-built subtrees in one propagate.
	pub completes_per_frame: u32,
}

impl LodChunkFulfillBudget {
	/// Retired HCSG hosts the `Last` despawn pass may recursive-despawn this frame.
	///
	/// Shares [`Self::cull_root_despawns_per_frame`] so playgrounds that raise
	/// cull teardown also drain Leave waves faster. At least one host so a
	/// zeroed cull cap cannot stall retirement.
	pub fn retired_host_despawns_per_frame(self) -> u32 {
		self.cull_root_despawns_per_frame.max(1)
	}

	/// Wall-clock stop for exclusive retired-host despawn after the first host.
	///
	/// Reuses [`Self::spawn_time_per_frame`] (default 2 ms), the same hitch
	/// clock exclusive chunk drain already honors.
	pub fn retired_host_time_per_frame(self) -> Duration {
		self.spawn_time_per_frame
	}
}

impl Default for LodChunkFulfillBudget {
	fn default() -> Self {
		Self {
			spawn_weights_per_frame: 512,
			spawn_time_per_frame: Duration::from_millis(2),
			max_atomic_spawn_cost: Duration::from_millis(3),
			cull_weights_per_frame: 64,
			cull_root_despawns_per_frame: 2,
			begins_per_frame: 48,
			begin_scan_per_frame: 192,
			begin_weights_per_frame: 512,
			begin_prefill_weights_per_job: 8,
			completes_per_frame: 512,
		}
	}
}

/// One exclusive semantic spawn quantum that exceeded [`LodChunkFulfillBudget::max_atomic_spawn_cost`].
#[derive(Debug, Clone, Copy)]
pub struct LodChunkAtomicOverrun {
	pub host: Entity,
	pub level: LodSceneLevel,
	pub elapsed: Duration,
}

/// Measured exclusive drain (always present; playgrounds / tests read it).
#[derive(Resource, Debug, Clone, Default)]
pub struct LodChunkDrainDiagnostics {
	/// Last exclusive drain wall time (pull + `World::spawn_scene`).
	pub last_exclusive_elapsed: Duration,
	/// Recent over-budget primitives (capped).
	pub atomic_overruns: Vec<LodChunkAtomicOverrun>,
}

impl LodChunkDrainDiagnostics {
	const MAX_OVERRUNS: usize = 16;

	pub(super) fn record_overrun(&mut self, overrun: LodChunkAtomicOverrun) {
		if self.atomic_overruns.len() >= Self::MAX_OVERRUNS {
			self.atomic_overruns.remove(0);
		}
		self.atomic_overruns.push(overrun);
	}
}

/// Remaining spawn / cull weight for the current frame.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkBudgetClock {
	pub spawn_remaining: u32,
	pub cull_remaining: u32,
}

/// Fulfill budget class: cold fill, desired upgrade, or shown warm-hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FulfillClass {
	#[default]
	Presence,
	Desired,
	Active,
}

/// Shared begin admission quotas for the current frame (all host `T`s).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkBeginClock {
	pub presence_remaining: u32,
	pub desired_remaining: u32,
	/// Reserved for symmetry with drain; begin rolls this into Desired.
	pub active_remaining: u32,
	/// Shared begin cost remaining ([`LodChunkFulfillBudget::begin_weights_per_frame`]).
	pub weight_remaining: u32,
	/// Which class the begin systems try first this frame.
	pub first_class: FulfillClass,
}

/// Round-robin cursor for the shared type-erased begin candidate scan.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LodChunkBeginScanCursor(pub u32);

/// Number of `(parent_band, self_band)` drain slots (5×5 High→Other).
pub const LOD_CHUNK_TUPLE_BAND_COUNT: usize = 25;

/// Round-robin cursors for `(parent, self)` drain tuple bands.
#[derive(Debug, Clone, Copy)]
pub struct LodChunkBandCursors {
	pub bands: [u32; LOD_CHUNK_TUPLE_BAND_COUNT],
}

impl Default for LodChunkBandCursors {
	fn default() -> Self {
		Self { bands: [0; LOD_CHUNK_TUPLE_BAND_COUNT] }
	}
}

/// Round-robin cursors + frame parity for drain scheduling.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkDrainCursor {
	pub frame: u64,
	pub presence: LodChunkBandCursors,
	pub desired: LodChunkBandCursors,
	pub active: LodChunkBandCursors,
}
