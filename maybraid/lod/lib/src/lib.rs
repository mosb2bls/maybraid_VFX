//! Umbrella crate for generalized LOD in Maybraid ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Re-exports [`lod_cascade`] as [`cascade`] and [`lod_cascade_system`] as [`cascade_system`] so dependents can pull in the stack from one dependency when convenient.

pub use lod_cascade as cascade;
pub use lod_cascade_system as cascade_system;

pub mod gen;
pub mod hcsg;
pub mod jobs;
pub mod lod_ref;
pub mod scene;

/// Compatibility module paths (prefer [`scene`] and [`lod_ref`]).
pub use scene::chunk as scene_chunk;
pub use scene::chunk_fulfill;
pub use scene::cull as lod_cull;
pub use scene::host as lod_scene_host;
pub use scene::level as lod_level;
pub use scene::refresh;
pub use scene::region_index;

pub use jobs::{ensure_lod_job_counter, LodJobCounter};
pub use lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, point_bounds, track_lod_nodes, FineLod,
	LodNode, LodNodeBounds, LodNodePlugin, LodNodePose, LodNodeSnapshot, LodNodeSystems, LodRef,
	LodRequest,
};
pub use scene::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, add_lod_refresh_cull_for,
	apply_lod_cull_requests, begin_chunk_lod_fulfill, begin_chunk_lod_fulfill_erased,
	cancel_unstarted_cull_for_desired_pending_roots,
	cancel_unstarted_cull_for_desired_pending_roots_erased, closest_available_lod_level,
	complete_chunk_lod_fulfill, cull_bands_with_adjacent_depth, cull_lod_level_roots,
	cull_named_from_factor, cull_non_adjacent_bands, cull_offset_bands,
	cull_offset_bands_from_factor, dominant_lod_ref, drain_chunk_lod_fulfill, drain_lod_cull,
	drain_lod_tree_vis, enqueue_lod_cull, fill_lod_cull_produce_cache, fill_lod_produce_cache,
	hide_lod_tree, hide_lod_tree_world, host_shows_level_root, lod_host_scene,
	lod_host_scene_pending, lod_level_roots_entity, lod_root_is_shown,
	lod_scene_host_or_ancestor_hidden, lod_world_entity_is_shown, named_band_index,
	named_band_progress, nested_host_parent_allows_refresh, parent_host_desired_or_high,
	produce_lod_cull_for_region, produce_lod_cull_for_region_erased, produce_lod_cull_regions,
	produce_lod_refresh_levels, produce_lod_refresh_levels_erased, produce_lod_refresh_regions,
	refresh_lod_host_levels, reset_lod_chunk_budget, settle_lod_level_root_visibility,
	show_lod_tree, sync_cullable_roots_marker, sync_lod_level_roots, sync_nested_refresh_allowed,
	update_lod_host_levels, Bullseye, LodChunk, LodChunkAtomicOverrun, LodChunkBeginScanCursor,
	LodChunkBudgetClock, LodChunkBudgetPlugin, LodChunkCullSystems, LodChunkDrainDiagnostics,
	LodChunkFulfillBudget, LodChunkFulfillSystems, LodChunkFulfillment, LodCullInFlight,
	LodCullMarkerPlugin, LodCullProduceCache, LodCullProduceCadence, LodCullRegionCursor,
	LodCullRegions, LodCullRegionsStatus, LodCullRequest, LodHostBounds, LodHostHasCullableRoots,
	LodLazyPending, LodLevelProduceSystems, LodLevelProducer, LodLevelRoot, LodLevelRootOverlap,
	LodLevelRootPending, LodLevelRootStreamed, LodLevelRoots, LodLevelSpawnRequest,
	LodNestedRefreshAllowed, LodNestedRefreshBlocked, LodNestedRefreshSyncBudget, LodProduceCache,
	LodRefreshCorePlugin, LodRefreshDomain, LodRefreshMembership, LodRefreshProductionPlugin,
	LodRefreshRegions, LodRefreshRegionsError, LodRefreshRegionsStatus, LodRefreshSystems,
	LodScene, LodSceneBoundsMarshaller, LodSceneCull, LodSceneCullAabb,
	LodSceneCullProduceFillPlugin, LodSceneCullRegion, LodSceneCullRegionPlugin, LodSceneCulls,
	LodSceneHost, LodSceneHostIndex, LodSceneHostPlugin, LodSceneHostStreamed, LodSceneLevel,
	LodSceneRefreshAabb, LodSceneRefreshEntitiesPlugin, LodSceneRefreshLevel,
	LodSceneRefreshLevelsFillPlugin, LodSceneRefreshLevelsPlugin, LodSceneRefreshPlugin,
	LodSceneRefreshRegion, LodSceneRefreshRegionPlugin, LodSceneRefreshSyncPlugin,
	LodSceneRegionCullPlugin, LodSceneRegionIndex, LodSceneStatus, LodTreeVisBudget,
	LodTreeVisQueue, LodViewer, OpenLattice, PatchSceneBounds, QuantizedDistance, SceneChunk,
	SemanticLodScene, SemanticSceneChunk, Spotlight, VisualLodPrimitive, VisualLodScene,
	VisualSceneChunk, DEFAULT_CHUNK_WEIGHT, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
