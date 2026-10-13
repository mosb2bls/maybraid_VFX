//! [`presentation<C, T>`]: marshals published `T` within channel `C`'s regions
//! into Bevy hosts.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::entity_disabling::Disabled;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec3;
use bevy::prelude::{
	Allow, App, Commands, CommandsSceneExt, Component, Entity, IntoScheduleConfigs, Local,
	MessageReader, Plugin, Query, Res, Transform, Update, With,
};

use super::node_store::StoredEntry;
use crate::gen::{Id, Version};
use crate::lod_ref::LodRef;
use crate::scene::{add_lod_refresh_chunk_for, lod_host_scene_pending, SemanticLodScene};

use super::bounds::{HcsgClass, HcsgRegions};
use super::context::GenerationScheme;
use super::demand::{HcsgDemand, SubscriptionId};
use super::node::HcsgNode;
use super::runtime::{ensure_runtime, HcsgSystems};
use super::storage::{Busy, HcsgStorage};

struct Host {
	entity: Entity,
	bounds: Aabb3d,
	version: Version,
	/// The session that last published this host's value.
	session: u64,
}

/// Per-system state for [`presentation`]: its subscription, how far it has
/// read, and the hosts it spawned.
pub struct Presented<T> {
	/// The channel's latest regions.
	wanted: Vec<Aabb3d>,
	focus: Option<Vec3>,
	class: HcsgClass,
	subscription: Option<SubscriptionId>,
	requested: Vec<Aabb3d>,
	cursor: usize,
	hosts: HashMap<Id, Host>,
	/// Regions hosts were last retired against; `None` until first run.
	retired_against: Option<Vec<Aabb3d>>,
	/// Advances when an epoch drops the subscription.
	session: u64,
	/// Hosts may remain from an earlier session.
	sweep_pending: bool,
	_t: PhantomData<fn() -> T>,
}

impl<T> Default for Presented<T> {
	fn default() -> Self {
		Self {
			wanted: Vec::new(),
			focus: None,
			class: HcsgClass::Near,
			subscription: None,
			requested: Vec::new(),
			cursor: 0,
			hosts: HashMap::new(),
			retired_against: None,
			session: 0,
			sweep_pending: false,
			_t: PhantomData,
		}
	}
}

/// Each frame: replace the subscription if `C`'s regions changed, spawn hosts
/// for newly published ids, and retire hosts that touch none of the regions.
/// Retired hosts despawn in `Last` ([`RetiredHost`]).
///
/// Across an epoch, hosts keep showing the previous session until its values
/// are replaced by version. Once the new session's subscription is done,
/// hosts it never published are retired.
///
/// Reads never wait on a lock: a busy store or demand defers to next frame.
/// Retiring a host does not evict its stored value; the worker evicts by reach
/// when the live subscription set changes. A presented value stays stored
/// because this system subscribes the same regions and type it keeps hosts for.
pub fn presentation<C, T>(
	mut regions: MessageReader<HcsgRegions<C>>,
	storage: Res<HcsgStorage>,
	demand: Res<HcsgDemand>,
	mut state: Local<Presented<T>>,
	mut commands: Commands,
) where
	C: Send + Sync + 'static,
	T: GenerationScheme + SemanticLodScene,
{
	if let Some(latest) = regions.read().last() {
		state.wanted = latest.boxes.clone();
		state.focus = latest.focus;
		state.class = latest.class;
	}
	state.request(&demand);
	state.spawn(&storage, &demand, &mut commands);
	state.retire(&mut commands);
}

impl<T: GenerationScheme + SemanticLodScene> Presented<T> {
	fn request(&mut self, demand: &HcsgDemand) {
		let wants = !self.wanted.is_empty();
		if self.wanted == self.requested && self.subscription.is_some() == wants {
			return;
		}
		self.requested = self.wanted.clone();
		self.cursor = 0;
		if wants {
			let regions = self.wanted.clone();
			self.subscription =
				Some(demand.subscribe::<T>(self.subscription, regions, self.focus, self.class));
		} else if let Some(subscription) = self.subscription.take() {
			demand.unsubscribe(subscription);
		}
	}

	fn spawn(&mut self, storage: &HcsgStorage, demand: &HcsgDemand, commands: &mut Commands) {
		let Some(subscription) = self.subscription else {
			return;
		};
		let published = match demand.try_read(subscription, self.cursor) {
			Ok(Some(published)) => published,
			Ok(None) => {
				self.subscription = None;
				self.session += 1;
				self.sweep_pending = !self.hosts.is_empty();
				return;
			}
			Err(Busy) => return,
		};
		let mut read_all = true;
		for id in published.ids {
			let Ok(entry) = storage.try_entry::<T>(id) else {
				read_all = false;
				break;
			};
			self.cursor += 1;
			let Some(entry) = entry else {
				continue;
			};
			if let Some(host) = self.hosts.get_mut(&id).filter(|host| host.version == entry.version)
			{
				host.session = self.session;
				continue;
			}
			if let Some(stale) = self.hosts.remove(&id) {
				retire_host(commands, stale.entity);
			}
			let (bounds, version) = (entry.bounds, entry.version);
			let entity = spawn_host(commands, id, entry, self.focus);
			self.hosts.insert(id, Host { entity, bounds, version, session: self.session });
			self.retired_against = None;
		}
		if read_all && published.done && self.sweep_pending {
			self.sweep_pending = false;
			let session = self.session;
			self.hosts.retain(|_, host| {
				let current = host.session == session;
				if !current {
					retire_host(commands, host.entity);
				}
				current
			});
		}
	}

	fn retire(&mut self, commands: &mut Commands) {
		if self.retired_against.as_ref() == Some(&self.wanted) {
			return;
		}
		self.retired_against = Some(self.wanted.clone());
		let wanted = &self.wanted;
		self.hosts.retain(|_, host| {
			let keep = wanted.iter().any(|region| region.intersects(&host.bounds));
			if !keep {
				retire_host(commands, host.entity);
			}
			keep
		});
	}
}

/// A host presentation has let go of. It despawns in `Last`, so commands other
/// systems queue on it, or on what it spawned, through `PostUpdate` still land.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RetiredHost;

fn retire_host(commands: &mut Commands, entity: Entity) {
	commands.entity(entity).try_insert(RetiredHost);
}

pub(super) fn despawn_retired_hosts(
	retired: Query<Entity, (With<RetiredHost>, Allow<Disabled>)>,
	mut commands: Commands,
) {
	for entity in &retired {
		commands.entity(entity).try_despawn();
	}
}

fn spawn_host<T: GenerationScheme + SemanticLodScene>(
	commands: &mut Commands,
	id: Id,
	entry: StoredEntry<Arc<T>>,
	focus: Option<Vec3>,
) -> Entity {
	let viewer = focus.map_or(Transform::IDENTITY, Transform::from_translation);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &entry.bounds,
	};
	let level = entry.value.scene_lod_level(&lod_ref);
	let node = HcsgNode { id, version: entry.version, bounds: entry.bounds, value: entry.value };
	commands
		.spawn_scene(lod_host_scene_pending(level, node.bounds))
		.insert(node)
		.id()
}

/// Presents `T` within channel `C`'s regions: requests its generation and
/// keeps one [`HcsgNode<T>`] host per published value.
///
/// By default also registers [`crate::scene::LodSceneRefreshChunkPlugin`] for
/// [`HcsgNode<T>`] once (shared across channels that present the same `T`).
/// Use [`PresentationPlugin::without_chunk_refresh`] when another refresh
/// path owns scene fulfillment (for example region refresh on bump-outs).
///
/// Something must produce `C`'s regions, such as [`super::HcsgBoundsPlugin`].
pub struct PresentationPlugin<C, T> {
	chunk_refresh: bool,
	_marker: PhantomData<fn() -> (C, T)>,
}

impl<C, T> Default for PresentationPlugin<C, T> {
	fn default() -> Self {
		Self { chunk_refresh: true, _marker: PhantomData }
	}
}

impl<C, T> PresentationPlugin<C, T> {
	/// Presentation only — no [`crate::scene::LodSceneRefreshChunkPlugin`] for
	/// [`HcsgNode<T>`].
	pub fn without_chunk_refresh() -> Self {
		Self { chunk_refresh: false, _marker: PhantomData }
	}
}

impl<C, T> Plugin for PresentationPlugin<C, T>
where
	C: Send + Sync + 'static,
	T: GenerationScheme + SemanticLodScene,
{
	fn build(&self, app: &mut App) {
		ensure_runtime(app);
		if self.chunk_refresh {
			add_lod_refresh_chunk_for::<HcsgNode<T>>(app);
		}
		app.add_message::<HcsgRegions<C>>()
			.add_systems(Update, presentation::<C, T>.in_set(HcsgSystems));
	}
}
