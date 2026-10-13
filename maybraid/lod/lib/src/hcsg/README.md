# HCSG generation and presentation

This is the design for the shared HCSG runtime. It has two aims:

1. **One generation path and one presentation path.** Each layer writes `GenerationScheme`s and scene implementations. The shared runtime provides two generic systems:
   - `generation<C, T>` keeps HCSG storage warm for type `T` within the regions sent on channel `C`.
   - `presentation<C, T>` does the same and also marshals the published values into Bevy entities.

   The runtime owns generation requests, storage access, the lifecycle of presentation hosts, and scene reconciliation. Layers no longer carry their own presenters, producers or window plumbing.
2. **No generation inside the frame.** Generation runs on a worker thread. The frame only publishes requested bounds and reads values that have already been published.

[#1005](https://github.com/ramate-io/maybraid/issues/1005) introduced `HcsgStorage`. [#1007](https://github.com/ramate-io/maybraid/issues/1007) moves the layers onto it. This document describes where both end up. Signatures here are illustrative.

## Overview

```text
 Bevy frame                         │  Generation worker (one thread)
                                    │
 generation::<C, T>                 │
   reads C's latest regions         │
   replaces its subscription ──────►│  (fire and forget)
                                    │
 presentation::<C, T>               │
   reads C's latest regions         │
   replaces its subscription ──────►│  Demand ── Condvar wakes the worker
   reads newly published ids  ◄─────│    discover T in the regions
   spawns / retires hosts           │    generate missing values (recursively)
   (HcsgNode<T> components)         │    publish to HcsgStorage
                                    │    append ids to the subscription
                                    │    sweep by reach when demand changes
 scene follow-on systems            │
   LodScene / VisualLodScene        │
   on HcsgNode<T>                   │
```

There are four parts, each with one job:

| Part | Owns |
|---|---|
| [Storage](#storage) | Completed, immutable values and their spatial index. |
| [Demand](#demand) | Subscriptions, deduplication of work, and waking the worker. |
| [Generation system](#the-generation-system) | Which values stay in HCSG for one channel and one type. |
| [Presentation](#presentation) | The active set of Bevy hosts for one channel and one type. |

Generation and presentation request work the same way. Presentation retires Bevy hosts. The worker evicts stored values by reach: each type is kept only within the regions of the live subscriptions that read it.

## Contract for generated values

A stored value is a **pure function of its key** `(TypeId, Id)` and of the session roots described below. Once published, a value never changes. Everything else follows from this rule:

- Values are shared as `Arc<T>`. Mutable gameplay state belongs in ECS components, never in stored values.
- Nothing published is ever corrected, so there is no general invalidation. Values stop being requested and can later be evicted, but they are never wrong.
- A scheme reads only through its generation context. It never reads ECS resources or other mutable state.

### Intent belongs in types

Different game modes ask for different things by asking for different **types**, not by mutating shared inputs. A mode that needs a different discovery or composition wraps a shared core:

```rust
/// Shared core, unaware of the mode: cached across modes.
impl GenerationScheme for Terrain { /* ... */ }

/// Mode-specific: the same core, composed differently for this mode.
struct Discovery<T>(PhantomData<T>);
impl<T: GenerationScheme> GenerationScheme for Discovery<T> { /* delegates, then adds */ }
```

Only types whose inputs really differ are wrapped. Shared lower types, such as Durham terrain, stay unaware of the mode, so their values carry over when the mode changes. Wrapper types, associated types and blanket impls express what is actually going on: a shared core behaving slightly differently in a given context.

### Session roots

Some values are chosen at runtime, such as a seed, a map or authored placements. These cannot be type parameters, so they enter as **seeded roots**:

1. **Initialization phase.** A mode seeds its roots when its session starts, before it subscribes anything.
2. **Immutable for the session.** Roots never change while the session runs.
3. **Mode exit** clears the stores of the mode's own wrapped types and bumps the [epoch](#sessions-and-epochs). The shared core stays cached.

Changing a root means starting a new session. Development-only knobs, such as playground reseeds, do the same thing: clear the affected stores and start again.

### The presented type is total

A layer presents one type, and that type exists for every cell in its bounds. For example, urbanization presents padded terrain for every ground cell. Where no pad reaches, the padded cell just wraps the ground cell. Durham's own terrain is not presented underneath it. This removes the need for any replacement machinery, such as hiding raw terrain under padded terrain.

## Storage

`HcsgStorage` keeps one typed store per generated type behind an erased registry. The whole store is downcast, while individual values stay typed. The effective key is `(TypeId, Id)`.

```rust
#[derive(Clone, Resource)]
pub struct HcsgStorage(Arc<Registry>);

struct Registry {
    stores: RwLock<HashMap<TypeId, Arc<dyn ErasedStore>>>,
    base_scales: RwLock<HashMap<TypeId, DVec3>>,
    next_version: AtomicU64, // one counter for every store
}

struct TypedStore<T> {
    nodes: RwLock<NodeStore<Arc<T>>>, // entries, gimme index, membership revision
}

pub struct StoredEntry<T> {
    pub value: T, // Arc<T> here
    pub bounds: Aabb3d,
    pub version: Version,
}
```

`TypedStore` reuses today's `NodeStore` (entries plus the gimme index) behind one lock. `HcsgValue` (renamed from today's `HcsgNode` trait) bounds what can be stored: `Send + Sync + 'static`.

### Locking

- The registry lock is held only long enough to clone the store's `Arc`. The guard is released before the store is touched.
- A store's lock is held only for lookup and publication. It is **never** held during generation or during recursive dependency calls.
- A value and its spatial entry are published together, under one write.
- The main thread uses `try_read` only (`try_entry`, `try_overlapping`, `try_membership_revision`). A failed read returns `Err(Busy)`, which means "nothing new this frame", never "empty". Blocking reads (`get`, `entry`, `overlapping`, `membership_revision`, `contains`) are `pub(crate)` inside `lod` (generation worker and unit tests) and, outside `lod`, only with the `test-support` feature.
- A poisoned lock is read through. Values are immutable and published whole, so the store is still consistent.

## Generation

### Schemes

Schemes receive a context instead of `&mut HcsgStorage`:

```rust
pub trait GenerationScheme: HcsgValue + Sized {
    /// Ids that originate in `region`. May generate dependencies through `cx`.
    fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>;

    /// The value for `id` and its bounds, or `None` where nothing exists.
    fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)>;
}
```

The context hands out owned references, so dependencies stay usable after storage locks are released:

```rust
impl GenerationContext {
    pub fn get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>>;
    pub fn get_or_generate<T: GenerationScheme>(&mut self, id: Id) -> Option<Arc<T>>;
    /// `T`'s origins in `region`, in id order, skipping any that don't build.
    pub fn get_or_generate_in<T: GenerationScheme>(&mut self, region: Aabb3d) -> Vec<Arc<T>>;
    /// Like `get_or_generate_in`, but `None` if any origin doesn't build.
    pub fn get_or_generate_all_in<T: GenerationScheme>(&mut self, region: Aabb3d) -> Option<Vec<Arc<T>>>;
    pub fn overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id>;
}
```

`get_or_generate` looks the value up, and if it is missing, builds it (recursively resolving that value's dependencies) and publishes it before returning. The context tracks which `(TypeId, Id)` pairs it is currently generating, and treats a repeat as a cycle (`None`).

Region pulls read origins only, never whatever happens to be stored in the region, so a value doesn't depend on which of its neighbors the worker built first.

The worker builds its context with a staleness check (`GenerationContext::with_stale`). Once the subscription is cancelled, the context generates nothing more and drops whatever it just built instead of publishing it. The final check happens under the store's write lock (`HcsgStorage::publish_unless`), so a value finished just as its session ends either lands before that session's stores are cleared or is dropped.

Schemes are synchronous. Large collections stay ordinary synchronous computations on the worker. Async generation can come later without changing the presentation boundary, because a partially constructed value stays private until it is complete.

### Demand

The demand layer sits between presentation and storage. It is the only shared state besides storage:

```rust
#[derive(Clone, Resource)]
pub struct HcsgDemand(Arc<DemandInner>);

struct DemandShared {
    next_id: AtomicU64,
    state: Mutex<DemandState>,
    wake: Condvar, // work arrived, or shutdown
    idle: Condvar, // the worker finished a subscription or ran dry
}

struct DemandState {
    subscriptions: HashMap<SubscriptionId, Subscription>,
    epoch: u64,
    working: bool,
    shutdown: bool,
}

struct Subscription {
    regions: Vec<Aabb3d>,
    focus: Option<Vec3>,
    class: HcsgClass,
    pass: f64,
    discover: Discover, // fn pointers monomorphized per T
    generate: Generate,
    discovered: Option<Vec<Id>>, // set after the first quantum
    cursor: usize,
    published: Vec<Id>, // appended as each value becomes available
    done: bool,
    cancelled: Arc<AtomicBool>, // set on replace, unsubscribe, or epoch end
}
```

The presentation side has three operations:

```rust
impl HcsgDemand {
    /// Removes `previous` (if any) and inserts a fresh subscription, under one lock.
    /// Notifies the worker.
    pub fn subscribe<T: GenerationScheme>(
        &self,
        previous: Option<SubscriptionId>,
        regions: Vec<Aabb3d>,
        focus: Option<Vec3>,
        class: HcsgClass,
    ) -> SubscriptionId;

    /// Ids published for `id` from `cursor` on, and whether the worker has
    /// finished the subscription. `Err(Busy)` if the lock is held; `Ok(None)`
    /// if the subscription no longer exists.
    pub fn try_read(&self, id: SubscriptionId, cursor: usize) -> Result<Option<Published>, Busy>;

    /// Undiscovered subscriptions plus remaining discovered ids in `classes`.
    /// Finished subscriptions do not count, including a panic on first
    /// discovery. First-load unveil uses this for Near; long-tail channels
    /// do not hold it.
    pub fn try_outstanding(&self, classes: &[HcsgClass]) -> Result<Outstanding, Busy>;

    pub fn unsubscribe(&self, id: SubscriptionId);
}

pub struct Published {
    pub ids: Vec<Id>,
    pub done: bool, // `ids` reaches the end of what will be published
}
```

`try_read_published` is the same read without `done`.

Subscription ids come from the `AtomicU64`. A presentation system holds exactly one subscription id at a time. Replacing it is a single locked operation, so subscriptions never leak.

### Worker

There is one dedicated worker thread. It schedules live subscriptions by
[`HcsgClass`](shared/bounds.rs) weight (stride `pass`, lowest first, newest
among ties) and runs each for a quantum of about 32 ids or 30 ms:

```text
loop:
  lock demand; wait on Condvar until some subscription is not done
  pick the lowest-pass unfinished subscription (newest id on a tie)
  fill one quantum:
    discover once (deduped, nearest to focus first); store ids and a cursor
    generate until 32 ids or 30 ms, then return to the scheduler
  advance that subscription's pass by cost / weight (a replacement inherits pass)
```

- **Every** discovered id whose value exists is appended to `published`, including values published earlier by another subscription. That's how presentation learns about values it didn't cause.
- **Cancellation is cheap.** Replacing, unsubscribing or ending the epoch removes the subscription and sets its `cancelled` flag. The worker checks the flag without taking the demand lock, and stops at the next id or dependency.
- **Deduplication.** With one worker, the published-value check in `get_or_generate` is enough: nothing else generates concurrently. The context's own generating set guards recursion. A cross-worker in-progress set waits for [more workers](#later).
- **A panicking scheme** is caught and logged. Its subscription is marked done with whatever was published so far, and the worker carries on.
- **Regions are coalesced.** Each generation or presentation system has one live subscription, holding its channel's latest regions. A camera sweep replaces subscriptions instead of queuing work behind them.

### Bounds sources

Regions arrive on typed channels. A producer `B` sends a set of boxes as `HcsgRegions<B>` messages, and every generation or presentation system registered on channel `B` responds to the latest set. A producer might follow a camera, a gameplay region or an explicit warming region:

```rust
pub trait HcsgBounds: Send + Sync + 'static {
    const CLASS: HcsgClass;
    type Param: SystemParam;
    /// The boxes to fill (and, for presentation, to keep hosts in).
    fn regions(param: &SystemParamItem<Self::Param>) -> Vec<Aabb3d>;
    fn focus(param: &SystemParamItem<Self::Param>) -> Option<Vec3> { None }
}

app.add_plugins(HcsgBoundsPlugin::<B>::default()); // once per channel
```

- `HcsgBoundsPlugin<B>` sends only when the boxes change, so a producer should snap its boxes to the cells it covers rather than follow the camera exactly. Focus alone never resubscribes.
- A shape that isn't a box is a set of boxes. A far terrain ring is four strips around its hole; joined, they are the annulus.
- An empty set requests nothing: the systems unsubscribe and presentation retires every host.
- `Gated<G, B>` is `B`'s regions while gate `G` is open, and none otherwise. It is its own channel, so a mode subscribes its layers to `Gated<ModeGate, B>` and closing the gate retires them. Discovery's `InDiscovery<B>` is one.
- Bounds sources that should share hosts must send on one channel.

### The generation system

`generation<C, T>` is presentation without the marshalling. It works the same way on the request side, but it has nothing to reconcile, so it is fire and forget at both ends:

```rust
pub fn generation<C, T: GenerationScheme>(
    mut regions: MessageReader<HcsgRegions<C>>,
    demand: Res<HcsgDemand>,
    mut state: Local<Generated<T>>,
) { /* ... */ }

struct Generated<T> {
    wanted: Vec<Aabb3d>,
    focus: Option<Vec3>,
    subscription: Option<SubscriptionId>,
    requested: Vec<Aabb3d>,
    _t: PhantomData<fn() -> T>,
}
```

- **Request:** when channel `C`'s regions change, it replaces its subscription, exactly as presentation does. It also resubscribes when `HcsgDemand::try_is_live` reports its subscription gone, for example after an epoch change. That is all it does on this side. It never reads `published`, keeps no cursor, holds no hosts and creates no entities.

Use it to keep values warm where nothing is presented yet, such as ahead of the camera or under a gameplay region. A type that is presented needs no separate generation system, because presentation requests its own generation.

### Retention

Stored values are a cache. A value is a pure function of its key, so an evicted value regenerates identically on demand. Retention is not the union of every live region: Geneva's language channel stays within 40 km of the viewer, and that union would pin the world.

Each type `U` is retained within the regions of the live subscriptions whose **reach** includes `U`:

1. **Record reach.** `GenerationContext` records the `TypeId` of every `get_or_generate`, `get`, and `entry` call, hit or miss. A subscription's reach is its own scheme's type plus everything its fills touched. A replacement shares that set, so a mid-quantum finish still lands on the live chain.
2. **Sweep when demand changes.** The worker sweeps between jobs, and only after the live set has changed (subscribe, replacement, unsubscribe, or epoch). For each stored type `U`, the retention set is the union of those regions, expanded by `U`'s configured base scale (one bucket of hysteresis). Entries whose stored bounds overlap none of them are removed. A type no live subscription reaches is cleared. `Id::Universal` is never swept; only a session reset clears those roots.
3. **Best effort.** Eviction order can still be off in either direction: a dependency may leave while values built from it stay, or a dependent may leave while its inputs stay. Neither affects correctness. [Propagating retention extents through the stack](https://github.com/ramate-io/maybraid/issues/1055) is follow-on work if rebuild churn warrants it.

Set `MAYBRAID_HCSG_DIAG` to log per-type store sizes and rebuild-after-eviction counts every few seconds while walking Discovery.

Presentation subscribes the same regions and type it keeps hosts for, so a presented value is always inside its own retention set. Hosts hold their own `Arc<T>` regardless.

## Presentation

### The host component

```rust
#[derive(Component)]
pub struct HcsgNode<T: HcsgValue> {
    pub id: Id,
    pub version: Version,
    pub bounds: Aabb3d, // the stored bounds
    pub value: Arc<T>,
}

impl<T: HcsgValue> Clone for HcsgNode<T> { /* clones the Arc */ }
```

`T` does not need to be a Bevy component or `Clone`, and host construction requires no `Default`. A host is built from a completed record.

Hosts sit at the identity transform, as forest groves already do. A presented value's scenes are posed in world space, and its stored bounds are the host's `LodHostBounds`.

### The presentation system

```rust
pub fn presentation<C, T: GenerationScheme + SemanticLodScene>(
    mut regions: MessageReader<HcsgRegions<C>>,
    storage: Res<HcsgStorage>,
    demand: Res<HcsgDemand>,
    mut state: Local<Presented<T>>,
    mut commands: Commands,
) { /* ... */ }

struct Presented<T> {
    wanted: Vec<Aabb3d>, // the channel's latest regions
    focus: Option<Vec3>,
    subscription: Option<SubscriptionId>,
    requested: Vec<Aabb3d>,
    retired_against: Option<Vec<Aabb3d>>,
    cursor: usize,
    hosts: HashMap<Id, Entity>,
    _t: PhantomData<fn() -> T>,
}
```

Each frame:

1. **Request.** If the channel's regions changed, replace the subscription (`subscribe` with the previous id) and reset the cursor. The presentation system never calls discovery itself.
2. **Read.** `try_read(subscription, cursor)`. On `Err(Busy)`, nothing changes this frame. On `Ok(None)`, the subscription is gone (for example after an epoch change), so subscribe again; this starts a new session of hosts.
3. **Spawn.** For each new id, clone the entry's `Arc` under a short `try_read` of the store, then spawn a pending LOD host (`lod_host_scene_pending`, at the level `T` picks for the focus) carrying `HcsgNode<T>`. If the store is busy, the id is retried next frame and the cursor is not advanced past it. An id already hosted at the same version is skipped. One hosted at an older version, which only happens across sessions, has its host replaced.
4. **Sweep.** Once a new session's subscription is read through to `done`, retire every host that session did not publish. A new session leaves no value behind, so this is the only time presentation retires by what was published.
5. **Retire.** Retire any host whose entry bounds intersect none of the regions, along with its owned scene subtree. Within a session, retirement depends only on bounds, never on what discovery returned. Hosts are only rechecked when the regions change or new hosts were spawned.

Retiring marks the host `RetiredHost`. The runtime despawns retired hosts in `Last`, so commands other systems queue for the host through `PostUpdate` still land.

Notes:
- When the subscription is replaced, existing hosts stay. Ids the new subscription publishes that are already in `hosts` are skipped.
- An id that leaves the bounds while it is still being generated is a presentation-side concern. Its subscription is gone, so the worker stops, and anything it already published is simply never spawned.
- Retiring a host does not evict its stored value. The worker evicts by [reach](#retention) when the live subscription set changes. A presented value stays stored because presentation subscribes the same regions and type it keeps hosts for.
- ECS changes are issued only after the `Arc` handles have been cloned and every storage guard has been released.

### Scenes

Scene traits are implemented on the wrapper and forward to the value:

```rust
impl<T: LodScene + HcsgValue> LodScene for HcsgNode<T> {
    // Forward scene selection and construction to self.value.
}

impl<T: VisualLodScene + HcsgValue> VisualLodScene for HcsgNode<T> {
    // Forward visual selection and construction to self.value.
}
```

- `LodScene` builds the semantic ECS subtree. `VisualLodScene` builds visual representations, including packed geometry held outside archetype storage.
- Every method forwards except `scene_bounds`, which returns the node's stored bounds (see [the host component](#the-host-component)).
- Host construction inserts `HcsgNode<T>` itself. It never forwards to an inner host-construction method that inserts or clones `T`.
- Follow-on systems query `HcsgNode<T>` directly.
- Within a session values never change, so a version change only happens across sessions. Asynchronous scene work still checks the host's version before attaching its result. A change of LOD alone reuses the same value.

### Registration and ordering

```rust
app.add_plugins(HcsgBoundsPlugin::<WorldBounds>::default());
app.add_plugins(PresentationPlugin::<WorldBounds, Terrain>::default());
// Chunk fulfill for HcsgNode<Terrain> is registered by PresentationPlugin.

// Region-driven level refresh (optional):
app.add_plugins(GimmeLodSceneRefreshPlugin::<HcsgNode<Terrain>, TerrainRefresh, With<Camera>>::default());

// Only where values should stay warm without being presented:
app.add_plugins(GenerationPlugin::<AheadOfCamera, Terrain>::default());
```

- Presentation needs no separately registered generation system. It requests its own generation.
- The channel is a type parameter, not a plugin field. Several plugins may share one channel, and the app adds its producer once.
- A value presented on several channels at different cell sizes is wrapped per channel. Durham's `Streamed<R, T>` is `T` on stream `R`'s ring cells, so each ring discovers only its own lattice, and `StreamPresentationPlugin<C, R, T>` presents it.
- Either plugin initializes `HcsgStorage` and `HcsgDemand` if missing, and spawns the one `HcsgWorker`.
- `PresentationPlugin` registers `LodSceneRefreshChunkPlugin<HcsgNode<T>>` once by default; use `without_chunk_refresh` when another refresh path owns fulfillment. Layers may still add region refresh (`GimmeLodSceneRefreshPlugin`, …) typed on `HcsgNode<T>`.
- A host's level is picked once at spawn. If it should change as the viewer moves, the layer also adds a refresh region source (`LodSceneRefreshRegionPlugin` with a `LodRefreshRegions` strategy) feeding `GimmeLodSceneRefreshPlugin`. Without one, nothing re-evaluates the level.
- Both systems run in `HcsgSystems`, before `LodRefreshSystems::Track`. Bevy applies the deferred host spawns before the refresh chain sees them.

## Sessions and epochs

The demand layer holds an `epoch`:

- **Mode exit** bumps the epoch, removes the mode's subscriptions, and clears the mode's wrapped stores.
- `advance_epoch` removes every subscription and sets its `cancelled` flag, so the worker stops and never publishes a value generated under an older epoch.
- **Mode enter** runs its initialization phase (seeding roots) before its generation and presentation systems subscribe.

A restart is always in this order: advance the epoch, clear the stores, seed the roots. Cancelling first is what lets the clear be final (see [Schemes](#schemes)). Hosts keep showing the previous session's values until the new ones land and replace them by version.

Each layer's roots expose `reset` (clear its stores, seed its roots). Advancing the epoch belongs to whoever owns the session, because one epoch ends every layer's subscriptions:

- `durham-playground` calls `DurhamRoots::restart` (advance, then reset) on a seed or layout change.
- Discovery does the same on entering the mode: `WorldLayersPlugin` advances once, then resets every layer. Its producers are gated on the mode, so they send nothing until it streams.

Presentation plugins never restart sessions or infer one from resource changes; they only gate where presentation runs.

Nothing else is invalidated. Reach records the `TypeId`s a subscription's fills read, not per-value references. Per-value retention extents are [follow-on work](https://github.com/ramate-io/maybraid/issues/1055).

## Testing

Tests keep the real worker thread and wait for it explicitly:

```rust
impl HcsgDemand {
    /// Blocks until no subscription has outstanding work, or `timeout` passes
    /// (`false`). Needs a running worker.
    pub fn wait_idle(&self, timeout: Duration) -> bool;
}
```

A typical test seeds its roots, runs `app.update()` once so presentation subscribes, calls `wait_idle(…)`, then runs `app.update()` again to spawn hosts and asserts on them. The timeout turns a stuck worker into a failed assertion instead of a hung test.

Schemes themselves can be tested without a worker, by building a `GenerationContext` over a seeded storage and generating directly.

## Layer responsibilities

A layer provides:

- its generation types: schemes, plus wrappers for mode-specific intent;
- the stores it registers, with their base scales;
- its bounds sources;
- scene implementations for the types it presents;
- gameplay systems, which query `HcsgNode<T>` or read storage without blocking.

The shared runtime provides everything else.

## What this replaces

The legacy per-layer runtime is gone: `gen::runtime`, `GenerateOn`, producers and queues, `Seed` invalidation, bespoke spatial indexes, terrain replacement machinery, and synchronous generation in Bevy systems. Every app now generates and presents on this shared worker path. Session roots, epochs, and store clears on mode exit replace the old restart hooks.

Legacy per-layer presenters (Durham terrain and water, Richmond hosts, Chico, Barking, Maputo, mobs) still compile in place; a final pass removes those modules once nothing references them.

## Migration

The public API is `lod::hcsg`: `HcsgStorage`, `HcsgDemand`, `GenerationContext`, `GenerationScheme`, `HcsgNode<T>`, `HcsgBounds`, `generation`, `presentation`, and the bounds/presentation plugins.

0. **Shared storage, context, demand and worker** (done).
1. **Generation and presentation systems** (done).
2. **Durham, Richmond, Chico, Barking, Maputo** (done). Native `GenerationScheme` impls; composed world and `durham-playground` run on the worker.
3. **Native Durham and cells** (done). No adapter or legacy storage.
4. **Modes and removal** (done).
   - **7a.** Training ground removed (to be rebuilt later).
   - **7b.** Discovery runs on this runtime: gated bounds channels, streamed terrain rings, gameplay through `DurhamSurface`.
   - **7c.** Legacy runtime, indexes, and layer-stack generation machinery deleted. Retired playgrounds per [`maybraid/PLAYGROUNDS.md`](../../../PLAYGROUNDS.md) (`routing-playground`, `barking-playground`, `character-world-movements-playground`, `richmond-playground`). Leftover per-layer presenter modules come out in a final 7c pass.

## Later

- [Propagate HCSG retention extents through the generation stack](https://github.com/ramate-io/maybraid/issues/1055), if rebuild-after-eviction counts show the current best-effort sweep churns too much.
- More than one worker. This needs an in-progress `(TypeId, Id)` set in the demand state, so two workers never build the same value.
- Async generation for large collections.
