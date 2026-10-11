# Incremental LOD scene chunks

Amortize spawning a **semantic** level root across frames under a weight budget.

[`SemanticLodScene`](../lib/src/scene/lod_scene.rs) (`LodScene` is an alias) builds
[`SceneChunk`](../lib/src/scene/chunk.rs) = `LodChunk<Box<dyn Scene>>`. Drain
spawns those scenes in the main World.

[`VisualLodScene`](../lib/src/scene/lod_scene.rs) builds
[`VisualSceneChunk`](../lib/src/scene/chunk.rs) = `LodChunk<VisualLodPrimitive>`.
That tree is **not** a Bevy `Scene` and is **not** consumed by
`drain_chunk_lod_fulfill`. [#667](https://github.com/ramate-io/maybraid/issues/667)
fills the visual leaf; this crate only defines the fork.

## API

Existing:

```rust
fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static;
```

Optional override:

```rust
fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk;
```

Default: `SceneChunk::primitive(self.scene_with_level(...))` — one spawn unit (full scene build still happens up front unless overridden).

Visual (no consume plugin yet):

```rust
fn visual_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> VisualSceneChunk;
```

## Lifecycle

Within [`LodRefreshSystems::Fulfill`](../lib/src/scene/refresh/sync/chunk.rs): reset budget → cancel unstarted cull on desired pending roots → **begin** → **drain** → **complete**.

1. Sync inserts `LodLevelSpawnRequest` when the desired root is missing.
2. **Begin** admits new jobs under [`LodChunkFulfillBudget`](../lib/src/scene/refresh/sync/chunk/types.rs): count ([`begins_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs)), shared materialized weight ([`begin_weights_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs), sum of prefilled primitive weights), per-job prefill cap ([`begin_prefill_weights_per_job`](../lib/src/scene/refresh/sync/chunk/types.rs); tails stay [`SceneChunk::Lazy`](../lib/src/scene/chunk.rs) for drain), and a capped host scan ([`begin_scan_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs)). Classified candidates sort by viewer XZ distance. Jobs split **Presence** (~¼), **Desired** (~⅜), and **Active** (~⅜) warm-hold; frame parity rotates class order. Each admitted job creates a hidden `LodLevelRoot` + `LodLevelRootPending` and seeds `LodChunkFulfillment`.
3. **Drain** is an exclusive `&mut World` system ([`drain_chunk_lod_fulfill`](../lib/src/scene/refresh/sync/chunk/drain.rs)): each `Lazy::next` + `World::spawn_scene` is one timed quantum. Work stops when the frame's spawn-weight clock is empty **or** elapsed time reaches [`spawn_time_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs) (stop **before** the next `pull_primitive`). Within each class, jobs drain by `(parent_desired, self_level)` High→…. Quanta over [`max_atomic_spawn_cost`](../lib/src/scene/refresh/sync/chunk/types.rs) are recorded on [`LodChunkDrainDiagnostics`](../lib/src/scene/refresh/sync/chunk/types.rs).
4. **Complete** waits for nested [`LodSceneHostStreamed`](../lib/src/scene/host.rs) bookkeeping, then caps visibility swaps at [`completes_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs): removes pending, sets the root `Inherited`, hides sibling roots.
5. **Cull** drains ready non-desired roots under [`cull_weights_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs), with [`cull_root_despawns_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs) allowing a few heavy recursive despawns per frame. The desired level (including in-progress pending) is never culled.

Cancel: if the desired level changes, pending roots for other levels are despawned and their queues dropped.

## Registration

```rust
add_lod_refresh_chunk_full_for::<MyHost>(app); // viewer-distance level writes + chunk fulfill + full-scan cull
// or
add_lod_refresh_chunk_for::<MyHost>(app);      // chunk fulfill only (region / probe writes level elsewhere)
// or Gimme region stack via gimme_host! (see chico/src/view.rs)
```

Pending hosts use [`SemanticLodScene::host`](../lib/src/scene/lod_scene.rs) (core pending
shell + [`host_contents`](../lib/src/scene/lod_scene.rs)); chunk fulfill streams
[`scene_chunks_with_level`](../lib/src/scene/lod_scene.rs). Domain types override
`host_contents` only — do not re-stamp `lod_host_scene_pending`.

## Future: coalescing and compaction

Many tiny weights can dominate scheduling overhead. Planned follow-ups:

- **Coalescing** — merge adjacent cheap primitives (or subtrees under a weight floor) into one spawn unit before enqueue.
- **Compaction** — rewrite a deep `SubChunks` tree into a flatter primitive list so the drain loop stays cheap.

Lazy materialization of subtrees (factories evaluated only when the scheduler reaches them) is also intentionally out of scope for the first cut.

## Related

- [`LodChunk` / `SceneChunk` / `VisualSceneChunk`](../lib/src/scene/chunk.rs)
- [`chunk_fulfill`](../lib/src/scene/chunk_fulfill.rs)
- [Richmond CONTRIBUTING — LodScene](../../world/layers/urbanization/CONTRIBUTING.md#lodscene-on-buildings)
