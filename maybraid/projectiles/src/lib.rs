//! Query-only bolts and bullets: shapecast through Fixed / Animated and emit
//! each distinct contact along the flight.

use std::collections::HashMap;

use avian3d::prelude::*;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

/// Capsule bolt. Gravity off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoltSpec {
	pub length: f32,
	pub radius: f32,
	pub speed: f32,
	pub max_range: f32,
	pub max_age: f32,
	pub penetration: f32,
	pub color: Color,
}

impl Default for BoltSpec {
	fn default() -> Self {
		Self {
			length: 0.55,
			radius: 0.11,
			speed: 180.0,
			max_range: 36.0,
			max_age: 2.0,
			penetration: 0.85,
			color: Color::srgb(0.35, 0.95, 1.0),
		}
	}
}

/// Capsule bullet. Gravity on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulletSpec {
	pub length: f32,
	pub radius: f32,
	pub speed: f32,
	pub max_range: f32,
	pub max_age: f32,
	pub penetration: f32,
	pub color: Color,
}

impl Default for BulletSpec {
	fn default() -> Self {
		Self {
			length: 0.32,
			radius: 0.04,
			speed: 28.0,
			max_range: 42.0,
			max_age: 3.0,
			penetration: 0.25,
			color: Color::srgb(1.0, 0.72, 0.22),
		}
	}
}

/// Multiplier on path length spent inside this collider. Missing is `1.0`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PenetrationCost(pub f32);

impl Default for PenetrationCost {
	fn default() -> Self {
		Self(1.0)
	}
}

/// Path / time / through-solid budgets for a bolt or bullet.
#[derive(Component, Debug, Clone)]
pub struct Flight {
	pub origin: Vec3,
	pub last: Vec3,
	pub path: f32,
	pub through: f32,
	pub age: f32,
	pub max_range: f32,
	pub max_through: f32,
	pub max_age: f32,
	contacted: Vec<Entity>,
}

impl Flight {
	pub fn spawn(origin: Vec3, max_range: f32, max_through: f32, max_age: f32) -> Self {
		Self {
			origin,
			last: origin,
			path: 0.0,
			through: 0.0,
			age: 0.0,
			max_range,
			max_through,
			max_age,
			contacted: Vec::new(),
		}
	}

	pub fn exhausted(&self) -> bool {
		self.path >= self.max_range || self.through > self.max_through || self.age >= self.max_age
	}

	fn note_contact(&mut self, target: Entity) -> bool {
		if self.contacted.contains(&target) {
			return false;
		}
		self.contacted.push(target);
		true
	}
}

/// Keep an exhausted flight alive through `PostUpdate` so contact consumers can
/// still read components such as the weapon's damage payload.
#[derive(Component, Debug, Clone, Copy)]
struct ExpiredFlight;

/// Entity responsible for spawning a projectile.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileSource(pub Entity);

/// One distinct collider crossed by a flight. Impacts listen; flights do not spawn VFX.
#[derive(Message, Clone, Copy, Debug)]
pub struct ProjectileContact {
	pub projectile: Entity,
	pub source: Option<Entity>,
	pub target: Entity,
	pub point: Vec3,
	pub normal: Vec3,
}

/// Public schedule point for projectile shapecasts. Contact consumers order after
/// [`ProjectileSystems::Sweep`] instead of naming the private `tick_flights` system.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProjectileSystems {
	/// Shapecast active flights and write [`ProjectileContact`]s (`PostUpdate`).
	Sweep,
}

pub struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		app.init_resource::<ProjectileVisualCache>()
			.add_message::<ProjectileContact>()
			.configure_sets(PostUpdate, ProjectileSystems::Sweep.after(TransformSystems::Propagate))
			.add_systems(PostUpdate, tick_flights.in_set(ProjectileSystems::Sweep))
			.add_systems(Last, despawn_expired_flights);
	}
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ProjectileVisualKey([u32; 6]);

impl ProjectileVisualKey {
	fn new(length: f32, radius: f32, color: Color) -> Self {
		let color = color.to_linear();
		Self([
			length.to_bits(),
			radius.to_bits(),
			color.red.to_bits(),
			color.green.to_bits(),
			color.blue.to_bits(),
			color.alpha.to_bits(),
		])
	}
}

/// Shared render assets for identical ballistic specifications.
#[derive(Resource, Default)]
pub struct ProjectileVisualCache(
	HashMap<ProjectileVisualKey, (Handle<Mesh>, Handle<StandardMaterial>)>,
);

impl ProjectileVisualCache {
	fn get_or_insert(
		&mut self,
		length: f32,
		radius: f32,
		color: Color,
		meshes: &mut Assets<Mesh>,
		materials: &mut Assets<StandardMaterial>,
	) -> (Handle<Mesh>, Handle<StandardMaterial>) {
		self.0
			.entry(ProjectileVisualKey::new(length, radius, color))
			.or_insert_with(|| {
				(meshes.add(Capsule3d::new(radius, length)), materials.add(glow_material(color)))
			})
			.clone()
	}
}

/// Solid length along a step of `ds`.
pub fn through_length(
	ds: f32,
	start_inside: bool,
	end_inside: bool,
	forward_hit: Option<f32>,
	backward_hit: Option<f32>,
) -> f32 {
	if ds <= 1e-8 {
		return 0.0;
	}
	let clamp = |x: f32| x.clamp(0.0, ds);
	match (start_inside, end_inside) {
		(true, true) => ds,
		(false, false) => match (forward_hit, backward_hit) {
			(Some(enter), Some(leave)) => clamp(ds - enter - leave),
			_ => 0.0,
		},
		(false, true) => clamp(ds - forward_hit.unwrap_or(0.0)),
		(true, false) => match (backward_hit, forward_hit) {
			(Some(air), _) => clamp(ds - air),
			(None, Some(exit)) => clamp(exit),
			(None, None) => ds,
		},
	}
}

fn glow_material(color: Color) -> StandardMaterial {
	let glow = color.to_linear();
	StandardMaterial {
		base_color: color,
		emissive: LinearRgba::rgb(glow.red * 14.0, glow.green * 14.0, glow.blue * 14.0),
		unlit: true,
		..default()
	}
}

fn capsule_along_y(direction: Vec3, muzzle: Vec3, length: f32, radius: f32) -> Transform {
	let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
	let center = muzzle + direction * (radius + length * 0.5);
	Transform { translation: center, rotation, scale: Vec3::ONE }
}

/// Longest shapecast segment. Faster bolts split `last → pos` so thin colliders
/// are not skipped in one huge `cast_shape`.
pub const MAX_SWEEP_METERS: f32 = 2.0;
const MAX_SWEEP_STEPS: u32 = 24;

fn visit_sweep_segments(start: Vec3, end: Vec3, max_len: f32, mut visit: impl FnMut(Vec3, Vec3)) {
	let delta = end - start;
	let total = delta.length();
	if total <= 1e-5 {
		return;
	}
	let dir = delta / total;
	let steps = ((total / max_len).ceil() as u32).clamp(1, MAX_SWEEP_STEPS);
	let step = total / steps as f32;
	for index in 0..steps {
		let a = start + dir * (step * index as f32);
		let b = start + dir * (step * (index + 1) as f32);
		visit(a, b);
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PenetrationSpan {
	enter: f32,
	exit: f32,
	cost: f32,
}

#[derive(Clone, Copy, Debug)]
struct Crossing {
	entity: Entity,
	start_inside: bool,
	end_inside: bool,
	forward: f32,
	backward: f32,
}

impl Crossing {
	fn new(entity: Entity) -> Self {
		Self {
			entity,
			start_inside: false,
			end_inside: false,
			forward: f32::INFINITY,
			backward: f32::INFINITY,
		}
	}
}

/// How unique colliders are folded into [`PenetrationSpan`]s.
///
/// Production uses [`Self::Vectors`]. [`Self::Crossings`] is the hashmap
/// alternative kept so the ignored timing harness can compare both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpanMode {
	Vectors,
	#[allow(dead_code)]
	Crossings,
}

/// Reused across projectile sweep segments to avoid per-segment allocator churn.
struct SweepScratch {
	filter: SpatialQueryFilter,
	hits: Vec<ShapeHitData>,
	starts: Vec<Entity>,
	ends: Vec<Entity>,
	backward: Vec<ShapeHitData>,
	entities: Vec<Entity>,
	seen: EntityHashMap<u8>,
	crossings: Vec<Crossing>,
	index: EntityHashMap<usize>,
	spans: Vec<PenetrationSpan>,
}

impl Default for SweepScratch {
	fn default() -> Self {
		Self {
			filter: SpatialQueryFilter::from_mask([
				PhysicsInteractionLayer::Fixed,
				PhysicsInteractionLayer::Animated,
			]),
			hits: Vec::new(),
			starts: Vec::new(),
			ends: Vec::new(),
			backward: Vec::new(),
			entities: Vec::new(),
			seen: EntityHashMap::default(),
			crossings: Vec::new(),
			index: EntityHashMap::default(),
			spans: Vec::new(),
		}
	}
}

impl SweepScratch {
	fn prepare_filter(&mut self, projectile: Entity, source: Option<Entity>) {
		self.filter.excluded_entities.clear();
		self.filter.excluded_entities.insert(projectile);
		if let Some(source) = source {
			self.filter.excluded_entities.insert(source);
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn fill_hits(
		spatial: &SpatialQuery,
		collider: &Collider,
		start: Vec3,
		end: Vec3,
		rotation: Quat,
		filter: &SpatialQueryFilter,
		hits: &mut Vec<ShapeHitData>,
		sort: bool,
	) {
		hits.clear();
		let delta = end - start;
		let ds = delta.length();
		if ds <= 1e-5 {
			return;
		}
		let Ok(dir) = Dir3::new(delta) else {
			return;
		};
		let config = ShapeCastConfig::from_max_distance(ds);
		spatial.shape_hits_callback(collider, start, rotation, dir, &config, filter, |hit| {
			hits.push(hit);
			true
		});
		if sort {
			// Avian's all-hit query does not guarantee traversal order.
			hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
		}
	}

	fn collect_forward(
		&mut self,
		spatial: &SpatialQuery,
		collider: &Collider,
		start: Vec3,
		end: Vec3,
		rotation: Quat,
	) {
		let Self { filter, hits, .. } = self;
		Self::fill_hits(spatial, collider, start, end, rotation, filter, hits, true);
	}

	fn ensure_crossing(
		crossings: &mut Vec<Crossing>,
		index: &mut EntityHashMap<usize>,
		entity: Entity,
	) -> usize {
		if let Some(&idx) = index.get(&entity) {
			idx
		} else {
			let idx = crossings.len();
			index.insert(entity, idx);
			crossings.push(Crossing::new(entity));
			idx
		}
	}

	fn note_unique(entities: &mut Vec<Entity>, seen: &mut EntityHashMap<u8>, entity: Entity) {
		if seen.insert(entity, 0).is_none() {
			entities.push(entity);
		}
	}

	fn nearest_distance(hits: &[ShapeHitData], entity: Entity) -> Option<f32> {
		hits.iter()
			.filter(|hit| hit.entity == entity)
			.map(|hit| hit.distance)
			.min_by(|a, b| a.total_cmp(b))
	}

	fn push_span(
		spans: &mut Vec<PenetrationSpan>,
		costs: &Query<&PenetrationCost>,
		entity: Entity,
		enter: f32,
		exit: f32,
	) {
		if exit > enter + 1e-5 {
			spans.push(PenetrationSpan {
				enter,
				exit,
				cost: costs.get(entity).map_or(1.0, |cost| cost.0),
			});
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn collect_spans(
		&mut self,
		spatial: &SpatialQuery,
		collider: &Collider,
		start: Vec3,
		end: Vec3,
		rotation: Quat,
		costs: &Query<&PenetrationCost>,
		mode: SpanMode,
	) {
		match mode {
			SpanMode::Vectors => {
				self.collect_spans_vectors(spatial, collider, start, end, rotation, costs);
			}
			SpanMode::Crossings => {
				self.collect_spans_crossings(spatial, collider, start, end, rotation, costs);
			}
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn collect_spans_vectors(
		&mut self,
		spatial: &SpatialQuery,
		collider: &Collider,
		start: Vec3,
		end: Vec3,
		rotation: Quat,
		costs: &Query<&PenetrationCost>,
	) {
		self.spans.clear();
		let ds = start.distance(end);
		if ds <= 1e-5 {
			return;
		}

		{
			let Self { filter, starts, .. } = self;
			starts.clear();
			spatial.shape_intersections_callback(collider, start, rotation, filter, |entity| {
				starts.push(entity);
				true
			});
		}
		if self.starts.is_empty() && self.hits.is_empty() {
			return;
		}

		{
			let Self { filter, ends, .. } = self;
			ends.clear();
			spatial.shape_intersections_callback(collider, end, rotation, filter, |entity| {
				ends.push(entity);
				true
			});
		}
		{
			let Self { filter, backward, .. } = self;
			Self::fill_hits(spatial, collider, end, start, rotation, filter, backward, false);
		}

		self.entities.clear();
		self.seen.clear();
		for entity in self
			.starts
			.iter()
			.chain(self.ends.iter())
			.chain(self.hits.iter().map(|hit| &hit.entity))
			.chain(self.backward.iter().map(|hit| &hit.entity))
			.copied()
		{
			Self::note_unique(&mut self.entities, &mut self.seen, entity);
		}

		for entity in self.entities.iter().copied() {
			let enter = if self.starts.contains(&entity) {
				0.0
			} else {
				let Some(distance) = Self::nearest_distance(&self.hits, entity) else {
					continue;
				};
				distance.clamp(0.0, ds)
			};
			let exit = if self.ends.contains(&entity) {
				ds
			} else {
				let Some(air) = Self::nearest_distance(&self.backward, entity) else {
					continue;
				};
				(ds - air).clamp(0.0, ds)
			};
			Self::push_span(&mut self.spans, costs, entity, enter, exit);
		}
	}

	#[allow(dead_code, clippy::too_many_arguments)]
	fn collect_spans_crossings(
		&mut self,
		spatial: &SpatialQuery,
		collider: &Collider,
		start: Vec3,
		end: Vec3,
		rotation: Quat,
		costs: &Query<&PenetrationCost>,
	) {
		self.spans.clear();
		let ds = start.distance(end);
		if ds <= 1e-5 {
			return;
		}

		self.crossings.clear();
		self.index.clear();
		{
			let Self { filter, crossings, index, .. } = self;
			spatial.shape_intersections_callback(collider, start, rotation, filter, |entity| {
				let idx = Self::ensure_crossing(crossings, index, entity);
				crossings[idx].start_inside = true;
				true
			});
		}
		if self.crossings.is_empty() && self.hits.is_empty() {
			return;
		}

		{
			let Self { filter, crossings, index, .. } = self;
			spatial.shape_intersections_callback(collider, end, rotation, filter, |entity| {
				let idx = Self::ensure_crossing(crossings, index, entity);
				crossings[idx].end_inside = true;
				true
			});
		}
		{
			let Self { filter, crossings, index, .. } = self;
			let delta = start - end;
			let back_ds = delta.length();
			if back_ds > 1e-5 {
				if let Ok(dir) = Dir3::new(delta) {
					let config = ShapeCastConfig::from_max_distance(back_ds);
					spatial.shape_hits_callback(
						collider,
						end,
						rotation,
						dir,
						&config,
						filter,
						|hit| {
							let idx = Self::ensure_crossing(crossings, index, hit.entity);
							if hit.distance < crossings[idx].backward {
								crossings[idx].backward = hit.distance;
							}
							true
						},
					);
				}
			}
		}
		for hit in &self.hits {
			let idx = Self::ensure_crossing(&mut self.crossings, &mut self.index, hit.entity);
			if hit.distance < self.crossings[idx].forward {
				self.crossings[idx].forward = hit.distance;
			}
		}

		for crossing in &self.crossings {
			let enter = if crossing.start_inside {
				0.0
			} else if crossing.forward.is_finite() {
				crossing.forward.clamp(0.0, ds)
			} else {
				continue;
			};
			let exit = if crossing.end_inside {
				ds
			} else if crossing.backward.is_finite() {
				(ds - crossing.backward).clamp(0.0, ds)
			} else {
				continue;
			};
			Self::push_span(&mut self.spans, costs, crossing.entity, enter, exit);
		}
	}

	/// Sweep `flight.last → transform` and write contacts. Returns true when the
	/// remaining budget cannot cover the full step (caller marks [`ExpiredFlight`]).
	#[allow(clippy::too_many_arguments)]
	fn sweep_flight(
		&mut self,
		mode: SpanMode,
		spatial: &SpatialQuery,
		costs: &Query<&PenetrationCost>,
		contacts: &mut MessageWriter<ProjectileContact>,
		entity: Entity,
		flight: &mut Flight,
		transform: &Transform,
		collider: &Collider,
		source: Option<Entity>,
		dt: f32,
	) -> bool {
		let pos = transform.translation;
		let rotation = transform.rotation;
		self.prepare_filter(entity, source);
		let delta = pos - flight.last;
		let ds = delta.length();
		let allowed = allowed_step_distance(flight, ds, dt);
		let end = if ds <= 1e-5 { flight.last } else { flight.last + delta / ds * allowed };
		flight.age = (flight.age + dt).min(flight.max_age);
		flight.path += allowed;
		let mut budget_exhausted = false;
		visit_sweep_segments(flight.last, end, MAX_SWEEP_METERS, |start, end| {
			if budget_exhausted {
				return;
			}
			let segment = end - start;
			let segment_length = segment.length();
			let direction = segment / segment_length.max(1e-8);
			self.collect_forward(spatial, collider, start, end, rotation);
			self.collect_spans(spatial, collider, start, end, rotation, costs, mode);
			for hit in &self.hits {
				let to_hit = hit.distance.clamp(0.0, segment_length);
				let through_before = penetration_at(&self.spans, to_hit);
				if flight.through + through_before > flight.max_through {
					break;
				}
				if flight.note_contact(hit.entity) {
					contacts.write(ProjectileContact {
						projectile: entity,
						source,
						target: hit.entity,
						point: hit.point1,
						normal: hit.normal1.normalize_or(-direction),
					});
				}
			}
			flight.through += penetration_at(&self.spans, segment_length);
			budget_exhausted = flight.through > flight.max_through;
		});
		flight.last = pos;
		budget_exhausted || allowed < ds - 1e-5
	}
}

fn penetration_at(spans: &[PenetrationSpan], distance: f32) -> f32 {
	spans
		.iter()
		.map(|span| (distance.min(span.exit) - span.enter).max(0.0) * span.cost)
		.sum()
}

fn allowed_step_distance(flight: &Flight, ds: f32, dt: f32) -> f32 {
	let range = (flight.max_range - flight.path).max(0.0);
	let age_fraction =
		if dt <= 1e-8 { 1.0 } else { ((flight.max_age - flight.age) / dt).clamp(0.0, 1.0) };
	ds.min(range).min(ds * age_fraction)
}

/// Avian default gravity, used to rewind a late spawn along its arc.
pub const STANDARD_GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

/// Position and velocity `age` seconds after leaving `muzzle` along `direction`.
pub fn ballistic_after_age(
	muzzle: Vec3,
	direction: Vec3,
	speed: f32,
	gravity: f32,
	age: f32,
) -> (Vec3, Vec3) {
	let accel = STANDARD_GRAVITY * gravity;
	let velocity = direction * speed;
	(muzzle + velocity * age + accel * (0.5 * age * age), velocity + accel * age)
}

/// Spawn a sensor capsule along `direction` from `muzzle`.
///
/// [`Flight::last`] starts at the muzzle so the first sweep can enter a body
/// the capsule center already occupies.
#[allow(clippy::too_many_arguments)]
pub fn spawn_flight(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	visuals: &mut ProjectileVisualCache,
	muzzle: Vec3,
	direction: Vec3,
	length: f32,
	radius: f32,
	speed: f32,
	max_range: f32,
	max_through: f32,
	max_age: f32,
	color: Color,
	gravity: f32,
) -> Entity {
	spawn_flight_aged(
		commands,
		meshes,
		materials,
		visuals,
		muzzle,
		direction,
		length,
		radius,
		speed,
		max_range,
		max_through,
		max_age,
		color,
		gravity,
		0.0,
	)
}

/// Like [`spawn_flight`], but the capsule is already `age` seconds downrange.
///
/// [`Flight::last`] stays at the leave muzzle so the next sweep covers that arc.
#[allow(clippy::too_many_arguments)]
pub fn spawn_flight_aged(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	visuals: &mut ProjectileVisualCache,
	muzzle: Vec3,
	direction: Vec3,
	length: f32,
	radius: f32,
	speed: f32,
	max_range: f32,
	max_through: f32,
	max_age: f32,
	color: Color,
	gravity: f32,
	age: f32,
) -> Entity {
	let age = age.max(0.0);
	let (pos, vel) = ballistic_after_age(muzzle, direction, speed, gravity, age);
	let dir = vel.normalize_or(direction);
	let transform = capsule_along_y(dir, pos, length, radius);
	let collider = Collider::capsule(radius, length);
	let (mesh, material) = visuals.get_or_insert(length, radius, color, meshes, materials);
	let mut flight = Flight::spawn(muzzle, max_range, max_through, max_age);
	flight.age = age;
	let entity = commands
		.spawn((
			Name::new("projectile"),
			transform,
			Visibility::default(),
			Mesh3d(mesh),
			MeshMaterial3d(material),
			RigidBody::Dynamic,
			MassPropertiesBundle::from_shape(&collider, 1.0),
			collider,
			Sensor,
			PhysicsInteractionLayer::projectile_layers(),
			LockedAxes::ROTATION_LOCKED,
			LinearVelocity(vel),
			GravityScale(gravity),
			Restitution::ZERO,
			flight,
		))
		.id();
	commands.entity(entity).insert(GlobalTransform::from(transform));
	entity
}

fn tick_flights(
	time: Res<Time>,
	spatial: SpatialQuery,
	costs: Query<&PenetrationCost>,
	mut contacts: MessageWriter<ProjectileContact>,
	mut commands: Commands,
	mut flights: Query<(Entity, &mut Flight, &Transform, &Collider, Option<&ProjectileSource>)>,
	mut scratch: Local<SweepScratch>,
) {
	let dt = time.delta_secs();
	for (entity, mut flight, transform, collider, source) in &mut flights {
		let source = source.map(|source| source.0);
		if scratch.sweep_flight(
			SpanMode::Vectors,
			&spatial,
			&costs,
			&mut contacts,
			entity,
			&mut flight,
			transform,
			collider,
			source,
			dt,
		) || flight.exhausted()
		{
			commands.entity(entity).insert(ExpiredFlight);
		}
	}
}

fn despawn_expired_flights(mut commands: Commands, expired: Query<Entity, With<ExpiredFlight>>) {
	for entity in &expired {
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[derive(Resource, Default)]
	struct ExpiredObserved(bool);

	fn observe_expired_flight(
		expired: Query<(), With<ExpiredFlight>>,
		mut observed: ResMut<ExpiredObserved>,
	) {
		observed.0 = !expired.is_empty();
	}

	#[test]
	fn through_length_air_and_solid() {
		assert_eq!(through_length(1.0, false, false, None, None), 0.0);
		assert_eq!(through_length(1.0, true, true, None, None), 1.0);
		assert_eq!(through_length(0.0, true, true, None, None), 0.0);
	}

	#[test]
	fn through_length_enter_and_thin_wall() {
		assert!((through_length(1.0, false, true, Some(0.3), None) - 0.7).abs() < 1e-5);
		assert!((through_length(1.0, false, false, Some(0.4), Some(0.4)) - 0.2).abs() < 1e-5);
		assert!((through_length(1.0, true, false, Some(0.4), Some(0.6)) - 0.4).abs() < 1e-5);
	}

	#[test]
	fn ballistic_after_age_moves_along_the_bore() {
		let (pos, vel) = ballistic_after_age(Vec3::ZERO, Vec3::X, 10.0, 0.0, 0.2);
		assert!((pos - Vec3::X * 2.0).length() < 1e-5);
		assert!((vel - Vec3::X * 10.0).length() < 1e-5);
		let (drop, falling) = ballistic_after_age(Vec3::ZERO, Vec3::X, 10.0, 1.0, 1.0);
		assert!((drop.x - 10.0).abs() < 1e-5);
		assert!(drop.y < 0.0);
		assert!(falling.y < 0.0);
	}

	#[test]
	fn flight_exhausts_on_any_budget() {
		let mut flight = Flight::spawn(Vec3::ZERO, 10.0, 1.0, 2.0);
		assert!(!flight.exhausted());
		flight.path = 10.1;
		assert!(flight.exhausted());
		flight.path = 0.0;
		flight.through = 1.1;
		assert!(flight.exhausted());
		flight.through = 0.0;
		flight.age = 2.1;
		assert!(flight.exhausted());
	}

	#[test]
	fn long_step_splits_into_two_meter_segments() {
		let start = Vec3::ZERO;
		let end = Vec3::X * 8.0;
		let mut segments = Vec::new();
		visit_sweep_segments(start, end, MAX_SWEEP_METERS, |a, b| segments.push((a, b)));
		assert_eq!(segments.len(), 4);
		assert!((segments[0].0 - start).length() < 1e-5);
		assert!((segments[3].1 - end).length() < 1e-4);
		for (a, b) in &segments {
			assert!((a.distance(*b) - 2.0).abs() < 1e-4, "{a} {b}");
		}
	}

	#[test]
	fn short_step_stays_one_segment() {
		let mut n = 0;
		visit_sweep_segments(Vec3::ZERO, Vec3::Z * 0.5, MAX_SWEEP_METERS, |_, _| n += 1);
		assert_eq!(n, 1);
	}

	#[test]
	fn flight_reports_each_entity_once() {
		let mut flight = Flight::spawn(Vec3::ZERO, 10.0, 1.0, 2.0);
		let a = Entity::from_bits(7);
		let b = Entity::from_bits(8);
		assert!(flight.note_contact(a));
		assert!(!flight.note_contact(a));
		assert!(flight.note_contact(b));
	}

	#[test]
	fn step_is_clamped_to_range_and_age() {
		let mut flight = Flight::spawn(Vec3::ZERO, 10.0, 1.0, 2.0);
		flight.path = 8.0;
		assert!((allowed_step_distance(&flight, 5.0, 0.1) - 2.0).abs() < 1e-5);
		flight.path = 0.0;
		flight.age = 1.95;
		assert!((allowed_step_distance(&flight, 5.0, 0.1) - 2.5).abs() < 1e-5);
	}

	#[test]
	fn expired_flight_survives_post_update_consumers() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ExpiredObserved>()
			.add_systems(PostUpdate, observe_expired_flight)
			.add_systems(Last, despawn_expired_flights);
		let projectile = app.world_mut().spawn(ExpiredFlight).id();

		app.update();

		assert!(app.world().resource::<ExpiredObserved>().0);
		assert!(app.world().get_entity(projectile).is_err());
	}

	#[test]
	fn penetration_sums_only_solid_spans() {
		let spans = [
			PenetrationSpan { enter: 0.2, exit: 0.5, cost: 2.0 },
			PenetrationSpan { enter: 0.8, exit: 1.0, cost: 1.0 },
		];
		assert!((penetration_at(&spans, 0.1) - 0.0).abs() < 1e-5);
		assert!((penetration_at(&spans, 0.6) - 0.6).abs() < 1e-5);
		assert!((penetration_at(&spans, 0.9) - 0.7).abs() < 1e-5);
		assert!((penetration_at(&spans, 1.2) - 0.8).abs() < 1e-5);
	}

	#[test]
	fn sweep_returns_multiple_ordered_contacts() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.finish();
		let near = app
			.world_mut()
			.spawn((
				RigidBody::Static,
				Collider::cuboid(0.2, 1.0, 1.0),
				Transform::from_xyz(2.0, 0.0, 0.0),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		let far = app
			.world_mut()
			.spawn((
				RigidBody::Static,
				Collider::cuboid(0.2, 1.0, 1.0),
				Transform::from_xyz(4.0, 0.0, 0.0),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		app.update();

		let hits = app
			.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				let mut hits = Vec::new();
				SweepScratch::fill_hits(
					&spatial,
					&Collider::sphere(0.05),
					Vec3::ZERO,
					Vec3::X * 6.0,
					Quat::IDENTITY,
					&PhysicsInteractionLayer::Fixed.query_filter(),
					&mut hits,
					true,
				);
				hits
			})
			.expect("spatial query system should run");
		assert_eq!(hits.iter().map(|hit| hit.entity).collect::<Vec<_>>(), [near, far]);
		assert!(hits[0].distance < hits[1].distance);

		let overlap_hits = app
			.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				let mut hits = Vec::new();
				SweepScratch::fill_hits(
					&spatial,
					&Collider::sphere(0.05),
					Vec3::X * 2.0,
					Vec3::X * 6.0,
					Quat::IDENTITY,
					&PhysicsInteractionLayer::Fixed.query_filter(),
					&mut hits,
					true,
				);
				hits
			})
			.expect("spatial query system should run");
		assert_eq!(overlap_hits.iter().map(|hit| hit.entity).collect::<Vec<_>>(), [near, far]);
		assert!(overlap_hits[0].distance <= 1e-5);
	}

	fn sweep_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
			ProjectilesPlugin,
		));
		app.finish();
		app
	}

	fn spawn_wall(app: &mut App, x: f32) -> Entity {
		app.world_mut()
			.spawn((
				RigidBody::Static,
				Collider::cuboid(0.2, 1.0, 1.0),
				Transform::from_xyz(x, 0.0, 0.0),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id()
	}

	fn spawn_flight_along(app: &mut App, from: Vec3, to: Vec3, max_through: f32) -> Entity {
		app.world_mut()
			.spawn((
				Transform::from_translation(to),
				RigidBody::Dynamic,
				Collider::sphere(0.05),
				Sensor,
				PhysicsInteractionLayer::projectile_layers(),
				LinearVelocity(Vec3::ZERO),
				GravityScale(0.0),
				Flight::spawn(from, 100.0, max_through, 10.0),
			))
			.id()
	}

	fn drain_contacts(app: &mut App) -> Vec<ProjectileContact> {
		app.world_mut().resource_mut::<Messages<ProjectileContact>>().drain().collect()
	}

	#[test]
	fn tick_flights_emits_distance_ordered_contacts() -> anyhow::Result<()> {
		let mut app = sweep_app();
		let near = spawn_wall(&mut app, 2.0);
		let far = spawn_wall(&mut app, 4.0);
		app.update();
		spawn_flight_along(&mut app, Vec3::ZERO, Vec3::X * 6.0, 10.0);
		app.update();
		let contacts = drain_contacts(&mut app);
		anyhow::ensure!(contacts.iter().map(|c| c.target).collect::<Vec<_>>() == [near, far]);
		Ok(())
	}

	#[test]
	fn tick_flights_reports_each_target_once() -> anyhow::Result<()> {
		let mut app = sweep_app();
		let wall = spawn_wall(&mut app, 2.0);
		app.update();
		let projectile = spawn_flight_along(&mut app, Vec3::ZERO, Vec3::X * 3.0, 10.0);
		app.update();
		let first = drain_contacts(&mut app);
		anyhow::ensure!(first.iter().filter(|c| c.target == wall).count() == 1);
		{
			let mut flight = app
				.world_mut()
				.get_mut::<Flight>(projectile)
				.ok_or_else(|| anyhow::anyhow!("projectile flight"))?;
			flight.last = Vec3::ZERO;
		}
		app.update();
		let second = drain_contacts(&mut app);
		anyhow::ensure!(second.iter().all(|c| c.target != wall));
		Ok(())
	}

	#[test]
	fn penetration_budget_stops_later_contacts() -> anyhow::Result<()> {
		let mut app = sweep_app();
		let near = spawn_wall(&mut app, 2.0);
		let far = spawn_wall(&mut app, 4.0);
		app.update();
		spawn_flight_along(&mut app, Vec3::ZERO, Vec3::X * 6.0, 0.05);
		app.update();
		let contacts = drain_contacts(&mut app);
		anyhow::ensure!(contacts.iter().map(|c| c.target).collect::<Vec<_>>() == [near]);
		anyhow::ensure!(contacts.iter().all(|c| c.target != far));
		Ok(())
	}

	#[test]
	fn projectile_and_source_are_excluded() -> anyhow::Result<()> {
		let mut app = sweep_app();
		let source = app
			.world_mut()
			.spawn((
				RigidBody::Static,
				Collider::cuboid(0.4, 1.0, 1.0),
				Transform::from_xyz(0.0, 0.0, 0.0),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		let projectile = app
			.world_mut()
			.spawn((
				Transform::from_xyz(3.0, 0.0, 0.0),
				RigidBody::Dynamic,
				Collider::sphere(0.05),
				Sensor,
				PhysicsInteractionLayer::projectile_layers(),
				LinearVelocity(Vec3::ZERO),
				GravityScale(0.0),
				Flight::spawn(Vec3::ZERO, 100.0, 10.0, 10.0),
				ProjectileSource(source),
			))
			.id();
		app.update();
		let contacts = drain_contacts(&mut app);
		anyhow::ensure!(contacts.iter().all(|c| c.target != source && c.target != projectile));
		Ok(())
	}

	#[test]
	fn contact_consumers_run_after_sweep_set() -> anyhow::Result<()> {
		#[derive(Resource, Default)]
		struct SawContact(bool);

		fn consume(
			mut contacts: MessageReader<ProjectileContact>,
			expired: Query<(), With<ExpiredFlight>>,
			mut saw: ResMut<SawContact>,
		) {
			saw.0 = saw.0 || contacts.read().next().is_some() || !expired.is_empty();
		}

		let mut app = sweep_app();
		app.init_resource::<SawContact>()
			.add_systems(PostUpdate, consume.after(ProjectileSystems::Sweep));
		spawn_wall(&mut app, 2.0);
		app.update();
		spawn_flight_along(&mut app, Vec3::ZERO, Vec3::X * 3.0, 10.0);
		app.update();
		anyhow::ensure!(app.world().resource::<SawContact>().0);
		Ok(())
	}

	#[test]
	fn span_modes_agree_on_overlapping_colliders() -> anyhow::Result<()> {
		fn through_for(mode: SpanMode) -> anyhow::Result<f32> {
			let mut app = sweep_app();
			spawn_wall(&mut app, 1.5);
			spawn_wall(&mut app, 1.7);
			spawn_wall(&mut app, 3.0);
			app.update();
			let projectile = spawn_flight_along(&mut app, Vec3::ZERO, Vec3::X * 5.0, 10.0);
			app.world_mut()
				.run_system_once(
					move |spatial: SpatialQuery,
					      costs: Query<&PenetrationCost>,
					      mut contacts: MessageWriter<ProjectileContact>,
					      mut flights: Query<(
						Entity,
						&mut Flight,
						&Transform,
						&Collider,
						Option<&ProjectileSource>,
					)>| {
						let mut scratch = SweepScratch::default();
						for (entity, mut flight, transform, collider, source) in &mut flights {
							if entity != projectile {
								continue;
							}
							scratch.sweep_flight(
								mode,
								&spatial,
								&costs,
								&mut contacts,
								entity,
								&mut flight,
								transform,
								collider,
								source.map(|source| source.0),
								1.0 / 60.0,
							);
						}
					},
				)
				.map_err(|err| anyhow::anyhow!("{err}"))?;
			app.world()
				.get::<Flight>(projectile)
				.map(|flight| flight.through)
				.ok_or_else(|| anyhow::anyhow!("flight through"))
		}
		let vectors = through_for(SpanMode::Vectors)?;
		let crossings = through_for(SpanMode::Crossings)?;
		anyhow::ensure!((vectors - crossings).abs() < 1e-4, "{vectors} vs {crossings}");
		Ok(())
	}

	/// `nix develop --command bash -lc 'PRE_COMMIT_DISABLE=true cargo test -p projectiles --release span_mode_scratch_timing -- --ignored --nocapture'`
	#[test]
	#[ignore = "release micro-benchmark; run with --release --ignored --nocapture"]
	fn span_mode_scratch_timing() {
		use std::hint::black_box;
		use std::time::Instant;

		fn scene(walls: usize, overlap: bool) -> App {
			let mut app = sweep_app();
			for index in 0..walls {
				let x = if overlap { 2.0 + index as f32 * 0.08 } else { 1.5 + index as f32 * 1.25 };
				spawn_wall(&mut app, x);
			}
			app.update();
			app
		}

		fn run(mode: SpanMode, walls: usize, overlap: bool, projectiles: usize) -> (f64, f32) {
			#[derive(Resource)]
			struct Mode(SpanMode);
			#[derive(Resource, Default)]
			struct BenchScratch(SweepScratch);
			#[derive(Resource, Default)]
			struct Through(f32);

			let mut app = scene(walls, overlap);
			for index in 0..projectiles {
				let z = (index as f32 - projectiles as f32 * 0.5) * 0.2;
				spawn_flight_along(&mut app, Vec3::new(0.0, 0.0, z), Vec3::new(8.0, 0.0, z), 4.0);
			}
			app.insert_resource(Mode(mode))
				.init_resource::<BenchScratch>()
				.init_resource::<Through>();

			fn sweep_once(
				mode: Res<Mode>,
				spatial: SpatialQuery,
				costs: Query<&PenetrationCost>,
				mut contacts: MessageWriter<ProjectileContact>,
				mut flights: Query<(
					Entity,
					&mut Flight,
					&Transform,
					&Collider,
					Option<&ProjectileSource>,
				)>,
				mut scratch: ResMut<BenchScratch>,
				mut through: ResMut<Through>,
			) {
				through.0 = 0.0;
				for (entity, mut flight, transform, collider, source) in &mut flights {
					flight.last = Vec3::new(0.0, 0.0, transform.translation.z);
					flight.path = 0.0;
					flight.through = 0.0;
					flight.age = 0.0;
					scratch.0.sweep_flight(
						mode.0,
						&spatial,
						&costs,
						&mut contacts,
						entity,
						&mut flight,
						transform,
						collider,
						source.map(|source| source.0),
						1.0 / 60.0,
					);
					through.0 += flight.through;
				}
			}

			let warmup = Instant::now();
			for _ in 0..8 {
				let _ = app.world_mut().run_system_once(sweep_once);
			}
			black_box(warmup.elapsed());
			let timed = Instant::now();
			const ITERS: u32 = 40;
			for _ in 0..ITERS {
				let _ = app.world_mut().run_system_once(sweep_once);
			}
			let elapsed = timed.elapsed().as_secs_f64() / ITERS as f64;
			(elapsed * 1e6, app.world().resource::<Through>().0)
		}

		for (label, walls, overlap) in
			[("empty", 0, false), ("sparse", 4, false), ("dense-overlap", 24, true)]
		{
			let (vectors_us, vectors_through) = run(SpanMode::Vectors, walls, overlap, 32);
			let (crossings_us, crossings_through) = run(SpanMode::Crossings, walls, overlap, 32);
			eprintln!(
				"span_mode_scratch_timing {label}: vectors {:.2} µs/iter (through {vectors_through:.2}), crossings {:.2} µs/iter (through {crossings_through:.2})",
				vectors_us, crossings_us
			);
		}
	}
}
