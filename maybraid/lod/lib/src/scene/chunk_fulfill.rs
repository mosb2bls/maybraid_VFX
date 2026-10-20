//! Re-export: chunk fulfill lives under [`crate::scene::refresh::sync`].

pub use crate::scene::refresh::sync::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, apply_lod_cull_requests,
	begin_chunk_lod_fulfill_erased, cancel_unstarted_cull_for_desired_pending_roots_erased,
	complete_chunk_lod_fulfill, drain_chunk_lod_fulfill, drain_lod_cull, enqueue_lod_cull,
	reset_lod_chunk_budget, LodChunkAtomicOverrun, LodChunkBeginScanCursor, LodChunkBudgetClock,
	LodChunkBudgetPlugin, LodChunkCullSystems, LodChunkDrainDiagnostics, LodChunkFulfillBudget,
	LodChunkFulfillSystems, LodChunkFulfillment, LodCullInFlight, LodCullRequest, LodLazyPending,
	LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed, LodSceneRefreshChunkPlugin,
	LodSceneRefreshSyncPlugin,
};
