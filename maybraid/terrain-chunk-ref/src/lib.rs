//! Lazy, shared references to terrain chunk meshes.
//!
//! [`TerrainChunkRef`] separates the identity of terrain geometry from any one presenter. Terrain,
//! ground-cover, and canopy entities can carry the same reference and receive the same
//! [`Handle<Mesh>`] while keeping independent materials. Fulfill stamps [`Mesh3d`] and the
//! chunk-minimum [`Transform`] — CpuShot vertices are local, so an identity pose piles the
//! mesh at the world origin.

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use lod_cascade::Chunk;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::mesh::handle::MeshHandle;
use render_item::mesh::{IdentifiedMesh, MeshBuilder, MeshFetcher, MeshId};
use render_item::NormalizeChunk;

/// Geometry identity for a resolved terrain mesh.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerrainChunkKey {
	pub mesh_id: MeshId,
	pub chunk: CascadeChunk,
}

/// A declarative request for one terrain model sampled over one LOD chunk.
///
/// Mesh-affecting options should be carried by `T` and represented by [`IdentifiedMesh::id`].
/// [`render_item::sdf::cpu_shot::CpuShotBuilder`] already includes wall-face options in that id.
#[derive(Component, Debug, Clone)]
pub struct TerrainChunkRef<T> {
	terrain_model: T,
	chunk: Chunk,
	res_2: u8,
	key: TerrainChunkKey,
}

impl<T> TerrainChunkRef<T> {
	pub fn chunk(&self) -> &Chunk {
		&self.chunk
	}

	/// Convert the engine-neutral footprint into the existing CPU-shot request type.
	pub fn cascade_chunk(&self) -> CascadeChunk {
		Self::cascade_chunk_for(&self.chunk, self.res_2)
	}

	fn cascade_chunk_for(chunk: &Chunk, res_2: u8) -> CascadeChunk {
		let origin = chunk.bounds_min();
		let extent = chunk.extent();
		CascadeChunk {
			world: 0,
			origin,
			size: extent.max_element(),
			extent: Some(extent),
			res_2,
			omit: chunk.omit(),
		}
	}

	/// Placement for CPU-shot terrain meshes, whose vertices are local to the chunk minimum.
	pub fn transform(&self) -> Transform {
		Transform::from_translation(self.chunk.bounds_min())
	}
}

impl<T> TerrainChunkRef<T>
where
	T: IdentifiedMesh + NormalizeChunk,
{
	pub fn new(terrain_model: T, chunk: Chunk, res_2: u8) -> Self {
		let cascade_chunk = Self::cascade_chunk_for(&chunk, res_2);
		let normalized = terrain_model.normalize_chunk(&cascade_chunk);
		let key = TerrainChunkKey { mesh_id: terrain_model.id(), chunk: normalized };
		Self { terrain_model, chunk, res_2, key }
	}

	pub fn key(&self) -> &TerrainChunkKey {
		&self.key
	}
}

/// Marker and key written after a [`TerrainChunkRef`] has supplied [`Mesh3d`].
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TerrainChunkRefResolved(pub TerrainChunkKey);

/// Marker for a successfully sampled chunk that contains no surface.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TerrainChunkRefEmpty(pub TerrainChunkKey);

/// Shared caches used by every [`TerrainChunkRef<T>`] in an app.
#[derive(Resource, Debug, Clone)]
pub struct TerrainChunkRefCache<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	handles: HandleMap<T>,
	disk: Option<DiskMeshCache<T>>,
	/// When false, a cache miss waits instead of marching. Use this when fill
	/// already owns [`Cached`] / `MeshDispatch` on the same [`HandleMap`].
	build_on_miss: bool,
}

impl<T> TerrainChunkRefCache<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	pub fn new() -> Self {
		Self { handles: HandleMap::new(), disk: None, build_on_miss: true }
	}

	pub fn with_disk_cache(mut self, disk: DiskMeshCache<T>) -> Self {
		self.disk = Some(disk);
		self
	}

	pub fn with_optional_disk(mut self, disk: Option<DiskMeshCache<T>>) -> Self {
		self.disk = disk;
		self
	}

	/// Share an existing [`HandleMap`]. Fill clones handles; the first miss still builds once
	/// unless [`Self::without_build_on_miss`] is set.
	///
	/// `MeshHandle::new` allocates a private map — inject this Arc so every
	/// [`TerrainChunkRef`] and any stacked `MeshDispatch` see the same mailbox.
	pub fn with_handles(mut self, handles: HandleMap<T>) -> Self {
		self.handles = handles;
		self
	}

	/// Overlay-only: copy a handle when fill has published one; never remarch.
	pub fn without_build_on_miss(mut self) -> Self {
		self.build_on_miss = false;
		self
	}

	pub fn handles(&self) -> HandleMap<T> {
		self.handles.clone()
	}

	pub fn cached_handle(&self, terrain_ref: &TerrainChunkRef<T>) -> Option<Handle<Mesh>> {
		self.handles.get_by_id(&terrain_ref.key.chunk, terrain_ref.key.mesh_id.clone())
	}
}

impl<T> Default for TerrainChunkRefCache<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

/// Last mailbox generation at which this ref looked up and missed.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TerrainChunkRefSeen {
	pub generation: u64,
	pub key: TerrainChunkKey,
}

/// Caps overlay applies and (optional) first-time builds per frame.
///
/// Cache hits consume [`Self::apply_per_frame`]. Cache probes and Seen stamps
/// consume [`Self::inspect_per_frame`]. [`Self::main_thread_budget`] starts
/// after the first query look. Overlay fill uses
/// [`TerrainChunkRefCache::without_build_on_miss`].
///
/// Waiters that already carry [`TerrainChunkRefSeen`] for the current
/// [`HandleMap`] generation are not iterated while the mailbox is quiet.
/// A generation bump re-walks those Seen waiters (budgeted). Without that
/// skip, a 2 ms budget was spent every idle frame walking already-Seen
/// overlays.
#[derive(Resource, Debug, Clone, Copy)]
pub struct TerrainChunkRefBudget {
	pub new_meshes_per_frame: u32,
	pub apply_per_frame: u32,
	/// Changed / unresolved refs to classify this frame.
	pub inspect_per_frame: u32,
	pub main_thread_budget: Duration,
}

impl Default for TerrainChunkRefBudget {
	fn default() -> Self {
		Self {
			new_meshes_per_frame: u32::MAX,
			apply_per_frame: 16,
			inspect_per_frame: 256,
			main_thread_budget: Duration::from_millis(2),
		}
	}
}

/// Resolve terrain references independently of their material type.
type TerrainChunkRefQueryItem<'a, T> = (
	Entity,
	&'a TerrainChunkRef<T>,
	Option<&'a TerrainChunkRefResolved>,
	Option<&'a TerrainChunkRefEmpty>,
	Option<&'a Mesh3d>,
	Option<&'a TerrainChunkRefSeen>,
);
/// New / changed / heal work. Excludes Seen waiters — those are only walked
/// when [`HandleMap::generation`] ticks (see [`fulfill_terrain_chunk_refs`]).
type TerrainChunkRefFreshFilter<T> = Or<(
	Changed<TerrainChunkRef<T>>,
	(
		Without<TerrainChunkRefResolved>,
		Without<TerrainChunkRefEmpty>,
		Without<TerrainChunkRefSeen>,
	),
	(With<TerrainChunkRefResolved>, Without<Mesh3d>),
)>;
type TerrainChunkRefSeenWaiterFilter = (
	With<TerrainChunkRefSeen>,
	Without<TerrainChunkRefResolved>,
	Without<TerrainChunkRefEmpty>,
);


/// Heal a wiped overlay pose. CpuShot verts are local to the chunk minimum.
pub fn sync_terrain_chunk_ref_poses<T>(mut query: Query<(&TerrainChunkRef<T>, &mut Transform)>)
where
	T: Send + Sync + 'static,
{
	for (terrain_ref, mut transform) in &mut query {
		let pose = terrain_ref.transform();
		if *transform != pose {
			*transform = pose;
		}
	}
}

pub fn fulfill_terrain_chunk_refs<T>(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	cache: Res<TerrainChunkRefCache<T>>,
	budget: Res<TerrainChunkRefBudget>,
	mut queries: ParamSet<(
		Query<TerrainChunkRefQueryItem<T>, TerrainChunkRefFreshFilter<T>>,
		Query<TerrainChunkRefQueryItem<T>, TerrainChunkRefSeenWaiterFilter>,
	)>,
	mut last_generation: Local<Option<u64>>,
) where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	let mut remaining = budget.new_meshes_per_frame;
	let mut applied = 0u32;
	let mut inspected = 0u32;
	let mut looked = 0u32;
	let started = Instant::now();
	let generation = cache.handles().generation();
	let rescan_seen = *last_generation != Some(generation);

	let mut budget_hit = false;
	for item in &queries.p0() {
		if !fulfill_one(
			item,
			&mut commands,
			&mut meshes,
			&cache,
			&budget,
			generation,
			&mut remaining,
			&mut applied,
			&mut inspected,
			&mut looked,
			started,
		) {
			budget_hit = true;
			break;
		}
	}
	if rescan_seen && !budget_hit {
		for item in &queries.p1() {
			if !fulfill_one(
				item,
				&mut commands,
				&mut meshes,
				&cache,
				&budget,
				generation,
				&mut remaining,
				&mut applied,
				&mut inspected,
				&mut looked,
				started,
			) {
				budget_hit = true;
				break;
			}
		}
		if !budget_hit {
			*last_generation = Some(generation);
		}
	} else if !rescan_seen {
		*last_generation = Some(generation);
	}
}

/// Returns `false` when a per-frame cap was hit and the caller should stop.
fn fulfill_one<T>(
	(entity, terrain_ref, resolved, empty, mesh, seen): TerrainChunkRefQueryItem<T>,
	commands: &mut Commands,
	meshes: &mut ResMut<Assets<Mesh>>,
	cache: &TerrainChunkRefCache<T>,
	budget: &TerrainChunkRefBudget,
	generation: u64,
	remaining: &mut u32,
	applied: &mut u32,
	inspected: &mut u32,
	looked: &mut u32,
	started: Instant,
) -> bool
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	if *looked > 0 && started.elapsed() >= budget.main_thread_budget {
		return false;
	}
	if *applied >= budget.apply_per_frame {
		return false;
	}
	*looked += 1;

	let key = terrain_ref.key();
	if mesh.is_some() && resolved.is_some_and(|resolved| &resolved.0 == key) {
		return true;
	}
	if empty.is_some_and(|empty| &empty.0 == key) {
		return true;
	}
	if seen.is_some_and(|seen| seen.generation == generation && seen.key == *key) {
		return true;
	}
	if *inspected >= budget.inspect_per_frame {
		return false;
	}
	*inspected += 1;

	if let Some(handle) = cache.cached_handle(terrain_ref) {
		apply_resolved_mesh(commands, entity, terrain_ref, handle);
		*applied += 1;
		return true;
	}
	commands.entity(entity).insert(TerrainChunkRefSeen { generation, key: key.clone() });
	if !cache.build_on_miss || *remaining == 0 {
		return true;
	}
	*remaining -= 1;

	let fetcher = MeshHandle::new(terrain_ref.terrain_model.clone())
		.with_handle_cache(cache.handles.clone())
		.with_mesh_cache(cache.disk.clone());
	if let Some(handle) = fetcher.fetch_mesh(meshes, &terrain_ref.cascade_chunk()) {
		apply_resolved_mesh(commands, entity, terrain_ref, handle);
		*applied += 1;
	} else {
		commands
			.entity(entity)
			.remove::<(Mesh3d, TerrainChunkRefResolved)>()
			.insert(TerrainChunkRefEmpty(key.clone()));
	}
	true
}

/// CpuShot verts are local to the chunk minimum. Insert pose with [`Mesh3d`] so
/// Bevy required-component defaults cannot leave an identity transform.
fn apply_resolved_mesh<T>(
	commands: &mut Commands,
	entity: Entity,
	terrain_ref: &TerrainChunkRef<T>,
	handle: Handle<Mesh>,
) where
	T: IdentifiedMesh + NormalizeChunk,
{
	commands
		.entity(entity)
		.remove::<(TerrainChunkRefEmpty, TerrainChunkRefSeen)>()
		.insert((
			Mesh3d(handle),
			TerrainChunkRefResolved(terrain_ref.key().clone()),
			terrain_ref.transform(),
		));
}

/// Installs shared handle caches and lazy fulfillment for one terrain model type.
pub struct TerrainChunkRefPlugin<T> {
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for TerrainChunkRefPlugin<T> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T> Plugin for TerrainChunkRefPlugin<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		app.init_resource::<TerrainChunkRefCache<T>>()
			.init_resource::<TerrainChunkRefBudget>()
			.add_systems(
				Update,
				(fulfill_terrain_chunk_refs::<T>, sync_terrain_chunk_ref_poses::<T>).chain(),
			);
	}
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::sync::Arc;

	use super::*;
	use render_item::mesh::cache::handle::map::HandleMap;

	#[derive(Clone)]
	struct CountingTerrain {
		builds: Arc<AtomicUsize>,
		ids: Arc<AtomicUsize>,
	}

	impl NormalizeChunk for CountingTerrain {}

	impl IdentifiedMesh for CountingTerrain {
		fn id(&self) -> MeshId {
			self.ids.fetch_add(1, Ordering::Relaxed);
			MeshId::new("counting-terrain".into())
		}
	}

	impl MeshBuilder for CountingTerrain {
		fn build_mesh_impl(&self, _chunk: &CascadeChunk) -> Option<Mesh> {
			self.builds.fetch_add(1, Ordering::Relaxed);
			Some(Mesh::from(Cuboid::from_length(1.0)))
		}
	}

	#[test]
	fn matching_refs_build_once_and_share_handle() -> anyhow::Result<()> {
		let builds = Arc::new(AtomicUsize::new(0));
		let ids = Arc::new(AtomicUsize::new(0));
		let model = CountingTerrain { builds: builds.clone(), ids: ids.clone() };
		let terrain_ref = TerrainChunkRef::new(model, Chunk::cube(Vec3::splat(-1.0), 2.0, None), 4);

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let a = app.world_mut().spawn(terrain_ref.clone()).id();
		let b = app.world_mut().spawn(terrain_ref).id();
		app.update();

		let a_mesh = app
			.world()
			.get::<Mesh3d>(a)
			.ok_or_else(|| anyhow::anyhow!("first terrain ref was not fulfilled"))?;
		let b_mesh = app
			.world()
			.get::<Mesh3d>(b)
			.ok_or_else(|| anyhow::anyhow!("second terrain ref was not fulfilled"))?;

		assert_eq!(builds.load(Ordering::Relaxed), 1);
		assert_eq!(a_mesh.0, b_mesh.0);
		let placed = Transform::from_translation(Vec3::splat(-1.0));
		assert_eq!(app.world().get::<Transform>(a).copied(), Some(placed));
		assert_eq!(app.world().get::<Transform>(b).copied(), Some(placed));
		let ids_after_fulfill = ids.load(Ordering::Relaxed);
		app.update();
		assert_eq!(ids.load(Ordering::Relaxed), ids_after_fulfill);
		Ok(())
	}

	#[test]
	fn fulfill_overwrites_identity_with_chunk_minimum() -> anyhow::Result<()> {
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let min = Vec3::new(160.0, -40.0, -320.0);
		let terrain_ref = TerrainChunkRef::new(model, Chunk::cube(min, 160.0, None), 4);
		let expected = terrain_ref.transform();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let entity = app.world_mut().spawn((terrain_ref, Transform::IDENTITY)).id();
		app.update();

		assert!(app.world().get::<Mesh3d>(entity).is_some());
		assert_eq!(app.world().get::<Transform>(entity).copied(), Some(expected));
		assert_ne!(expected, Transform::IDENTITY);
		Ok(())
	}

	#[test]
	fn sync_restores_wiped_overlay_pose() -> anyhow::Result<()> {
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let min = Vec3::new(160.0, -40.0, -320.0);
		let terrain_ref = TerrainChunkRef::new(model, Chunk::cube(min, 160.0, None), 4);
		let expected = terrain_ref.transform();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let entity = app.world_mut().spawn(terrain_ref).id();
		app.update();
		*app.world_mut()
			.get_mut::<Transform>(entity)
			.ok_or_else(|| anyhow::anyhow!("fulfilled ref lost Transform"))? = Transform::IDENTITY;
		app.update();

		assert_eq!(app.world().get::<Transform>(entity).copied(), Some(expected));
		Ok(())
	}

	#[test]
	fn resolution_changes_mesh_identity() {
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = Chunk::cube(Vec3::splat(-1.0), 2.0, None);
		let low = TerrainChunkRef::new(model.clone(), chunk, 3);
		let high = TerrainChunkRef::new(model, chunk, 5);

		assert_ne!(low.key(), high.key());
	}

	#[derive(Clone)]
	struct EmptyTerrain {
		builds: Arc<AtomicUsize>,
	}

	impl NormalizeChunk for EmptyTerrain {}

	impl IdentifiedMesh for EmptyTerrain {
		fn id(&self) -> MeshId {
			MeshId::new("empty-terrain".into())
		}
	}

	impl MeshBuilder for EmptyTerrain {
		fn build_mesh_impl(&self, _chunk: &CascadeChunk) -> Option<Mesh> {
			self.builds.fetch_add(1, Ordering::Relaxed);
			None
		}
	}

	#[test]
	fn empty_ref_does_not_rebuild_every_frame() {
		let builds = Arc::new(AtomicUsize::new(0));
		let terrain_ref = TerrainChunkRef::new(
			EmptyTerrain { builds: builds.clone() },
			Chunk::cube(Vec3::splat(-1.0), 2.0, None),
			4,
		);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(TerrainChunkRefPlugin::<EmptyTerrain>::default());
		app.world_mut().spawn(terrain_ref);

		app.update();
		app.update();

		assert_eq!(builds.load(Ordering::Relaxed), 1);
	}

	#[test]
	fn injected_handle_map_is_shared_across_cache_clones() {
		let handles = HandleMap::<CountingTerrain>::new();
		let cache = TerrainChunkRefCache::<CountingTerrain>::new().with_handles(handles.clone());
		let a = cache.handles();
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = CascadeChunk {
			world: 0,
			origin: Vec3::ZERO,
			size: 2.0,
			extent: Some(Vec3::splat(2.0)),
			res_2: 2,
			omit: None,
		};
		let mesh = Handle::default();
		a.insert(&chunk, &model, mesh.clone());
		assert!(handles.get(&chunk, &model).is_some());
	}

	#[test]
	fn apply_budget_spreads_cache_hits() -> anyhow::Result<()> {
		let handles = HandleMap::<CountingTerrain>::new();
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = Chunk::cube(Vec3::splat(-1.0), 2.0, None);
		let terrain_ref = TerrainChunkRef::new(model.clone(), chunk, 2);
		handles.insert(&terrain_ref.cascade_chunk(), &model, Handle::default());

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default())).init_asset::<Mesh>();
		app.insert_resource(
			TerrainChunkRefCache::<CountingTerrain>::new()
				.with_handles(handles)
				.without_build_on_miss(),
		)
		.insert_resource(TerrainChunkRefBudget {
			new_meshes_per_frame: 0,
			apply_per_frame: 1,
			inspect_per_frame: u32::MAX,
			main_thread_budget: Duration::from_secs(1),
		})
		.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let a = app.world_mut().spawn(terrain_ref.clone()).id();
		let b = app.world_mut().spawn(terrain_ref).id();
		app.update();
		let first = app.world().get::<Mesh3d>(a).is_some();
		let second = app.world().get::<Mesh3d>(b).is_some();
		assert_ne!(first, second, "one apply slot should stamp exactly one ref");
		app.update();
		assert!(app.world().get::<Mesh3d>(a).is_some());
		assert!(app.world().get::<Mesh3d>(b).is_some());
		Ok(())
	}

	#[test]
	fn inspect_budget_spreads_overlay_miss_scan() -> anyhow::Result<()> {
		let handles = HandleMap::<CountingTerrain>::new();
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = Chunk::cube(Vec3::splat(-1.0), 2.0, None);
		let terrain_ref = TerrainChunkRef::new(model.clone(), chunk, 2);

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default())).init_asset::<Mesh>();
		app.insert_resource(
			TerrainChunkRefCache::<CountingTerrain>::new()
				.with_handles(handles)
				.without_build_on_miss(),
		)
		.insert_resource(TerrainChunkRefBudget {
			new_meshes_per_frame: 0,
			apply_per_frame: 16,
			inspect_per_frame: 8,
			main_thread_budget: Duration::from_secs(1),
		})
		.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let entities: Vec<_> =
			(0..32).map(|_| app.world_mut().spawn(terrain_ref.clone()).id()).collect();
		app.update();
		let seen = entities
			.iter()
			.filter(|entity| app.world().get::<TerrainChunkRefSeen>(**entity).is_some())
			.count();
		assert_eq!(seen, 8, "inspect cap should stamp Seen on a slice, not the whole waiter set");
		assert!(entities.iter().all(|entity| app.world().get::<Mesh3d>(*entity).is_none()));
		assert_eq!(model.builds.load(Ordering::Relaxed), 0);

		for _ in 0..3 {
			app.update();
		}
		let seen = entities
			.iter()
			.filter(|entity| app.world().get::<TerrainChunkRefSeen>(**entity).is_some())
			.count();
		assert_eq!(seen, 32);

		// Quiet mailbox: further ticks must not drop Seen or remarch.
		for _ in 0..4 {
			app.update();
		}
		assert_eq!(
			entities
				.iter()
				.filter(|entity| app.world().get::<TerrainChunkRefSeen>(**entity).is_some())
				.count(),
			32
		);
		assert!(entities.iter().all(|entity| app.world().get::<Mesh3d>(*entity).is_none()));
		assert_eq!(model.builds.load(Ordering::Relaxed), 0);
		Ok(())
	}

	#[test]
	fn cache_only_fulfill_waits_for_an_injected_handle() -> anyhow::Result<()> {
		let handles = HandleMap::<CountingTerrain>::new();
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = Chunk::cube(Vec3::splat(-1.0), 2.0, None);
		let terrain_ref = TerrainChunkRef::new(model.clone(), chunk, 2);
		let cascade = terrain_ref.cascade_chunk();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default())).init_asset::<Mesh>();
		app.insert_resource(
			TerrainChunkRefCache::<CountingTerrain>::new()
				.with_handles(handles.clone())
				.without_build_on_miss(),
		)
		.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let entity = app.world_mut().spawn(terrain_ref.clone()).id();
		app.update();
		assert!(app.world().get::<Mesh3d>(entity).is_none());
		assert_eq!(model.builds.load(Ordering::Relaxed), 0);

		let mesh = Handle::default();
		handles.insert(&cascade, &model, mesh.clone());
		app.update();
		let resolved = app
			.world()
			.get::<Mesh3d>(entity)
			.ok_or_else(|| anyhow::anyhow!("fill handle was not copied"))?;
		assert_eq!(resolved.0, mesh);
		assert_eq!(model.builds.load(Ordering::Relaxed), 0);
		assert_eq!(app.world().get::<Transform>(entity).copied(), Some(terrain_ref.transform()));
		Ok(())
	}
}
