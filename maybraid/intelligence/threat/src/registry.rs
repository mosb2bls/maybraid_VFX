use std::collections::{BTreeMap, HashMap};

use bevy::math::{bounding::Aabb3d, DVec3};
use bevy::prelude::*;
use gimme_core::{BaseScale, HashMapStore, Level, SpatialId, SpatialIndexError, TypedIndex};

use crate::{Affiliations, ThreatId, ThreatSubject};

const BASE_SCALE: f64 = 64.0;
const MAX_QUERY_RADIUS: f32 = 1_000.0;

/// Current entity-backed threat candidate snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreatRecord {
	pub id: ThreatId,
	pub entity: Entity,
	pub position: Vec3,
	pub salience: f32,
	pub affiliations: Affiliations,
}

impl SpatialId for ThreatRecord {
	type Id = ThreatId;

	fn spatial_id(&self) -> Self::Id {
		self.id
	}
}

/// Gimme-backed local threat-subject index.
#[derive(Resource)]
pub struct ThreatRegistry {
	local: TypedIndex<ThreatRecord, HashMapStore<ThreatRecord>>,
	records: BTreeMap<ThreatId, ThreatRecord>,
	by_entity: HashMap<Entity, ThreatId>,
	max_level: Level,
}

impl Default for ThreatRegistry {
	fn default() -> Self {
		let local = match TypedIndex::new(DVec3::splat(BASE_SCALE), HashMapStore::new()) {
			Ok(index) => index,
			Err(error) => unreachable!("fixed positive threat base scale was rejected: {error}"),
		};
		Self { local, records: BTreeMap::new(), by_entity: HashMap::new(), max_level: 0 }
	}
}

impl ThreatRegistry {
	pub fn upsert(
		&mut self,
		entity: Entity,
		subject: ThreatSubject,
		affiliations: &Affiliations,
		position: Vec3,
	) -> Result<(), SpatialIndexError> {
		if self.records.get(&subject.id).is_some_and(|record| record.entity != entity)
			|| !position.is_finite()
			|| !subject.salience.is_finite()
		{
			self.remove_entity(entity);
			return Err(SpatialIndexError::InsertFailed);
		}
		if let Some(previous_id) = self.by_entity.get(&entity).copied() {
			if previous_id != subject.id {
				self.remove_id(previous_id);
			}
		}
		let record = ThreatRecord {
			id: subject.id,
			entity,
			position,
			salience: subject.salience.max(0.0),
			affiliations: affiliations.clone(),
		};
		let extent = Vec3::splat(0.05);
		let bounds = Aabb3d::from_min_max(position - extent, position + extent);
		let base = BaseScale::new(self.local.grid().base_scale())?;
		self.max_level = self.max_level.max(base.insertion_level(&bounds));
		self.local.remove_value(subject.id);
		self.local.insert_value(record.clone(), bounds)?;
		self.records.insert(subject.id, record);
		self.by_entity.insert(entity, subject.id);
		Ok(())
	}

	pub fn remove_entity(&mut self, entity: Entity) -> Option<ThreatRecord> {
		let id = self.by_entity.remove(&entity)?;
		self.remove_id(id)
	}

	pub fn get(&self, id: ThreatId) -> Option<&ThreatRecord> {
		self.records.get(&id)
	}

	pub fn get_entity(&self, entity: Entity) -> Option<&ThreatRecord> {
		self.by_entity.get(&entity).and_then(|id| self.records.get(id))
	}

	/// Fill `out` with ids of records within `radius` of `center`, sorted by id.
	///
	/// Avoids cloning [`ThreatRecord`] payloads; pair with [`Self::get`] on the hot path.
	pub fn collect_local(&self, center: Vec3, radius: f32, out: &mut Vec<ThreatId>) {
		out.clear();
		if !center.is_finite() || !radius.is_finite() {
			return;
		}
		let radius = radius.clamp(0.0, MAX_QUERY_RADIUS);
		let extent = Vec3::splat(radius);
		let region = Aabb3d::from_min_max(center - extent, center + extent);
		let query_level = BaseScale::new(self.local.grid().base_scale())
			.map(|base| base.insertion_level(&region))
			.unwrap_or(0);
		for (record, _) in self
			.local
			.query_values(region, BaseScale::levels_through(query_level.max(self.max_level)))
		{
			if center.distance(record.position) <= radius {
				out.push(record.id);
			}
		}
		out.sort_by_key(|id| *id);
	}

	pub fn local(&self, center: Vec3, radius: f32) -> Vec<ThreatRecord> {
		let mut ids = Vec::new();
		self.collect_local(center, radius, &mut ids);
		ids.iter().filter_map(|id| self.get(*id).cloned()).collect()
	}

	pub fn len(&self) -> usize {
		self.records.len()
	}

	pub fn is_empty(&self) -> bool {
		self.records.is_empty()
	}

	fn remove_id(&mut self, id: ThreatId) -> Option<ThreatRecord> {
		self.local.remove_value(id);
		let record = self.records.remove(&id)?;
		self.by_entity.remove(&record.entity);
		Some(record)
	}
}
