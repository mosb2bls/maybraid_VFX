//! Thrown grenade body, fuse, blast hits, and detonation message.

mod material;
mod sound;

use avian3d::prelude::*;
use bevy::prelude::*;
use character_items::{GrenadeSpec, GrenadeStats};
use damage::Hit;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

pub use material::{grenade_material, GrenadeMaterial, GrenadeMaterialPlugin};
pub use sound::{
	GrenadeSounds, BOUNCE_SPATIAL_RADIUS, CHARGE_IMPACT, CHARGE_RUMBLE, CHARGE_THROW,
	IMPACT_SPATIAL_RADIUS, RUMBLE_SPATIAL_RADIUS, THROW_SPATIAL_RADIUS,
};

/// Palette-free explosion knobs copied onto the thrown body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrenadeEffect {
	pub scale: f32,
	pub intensity: f32,
	pub playback: f32,
	pub seed: Option<u64>,
}

impl GrenadeEffect {
	pub fn from_stats(stats: &GrenadeStats) -> Self {
		Self {
			scale: stats.effect_scale,
			intensity: stats.effect_intensity,
			playback: stats.effect_playback,
			seed: None,
		}
	}
}

/// Independent world entity after release.
#[derive(Component, Debug)]
pub struct ThrownGrenade {
	pub spec: GrenadeSpec,
	pub stats: GrenadeStats,
	pub effect: GrenadeEffect,
	pub source: Entity,
	pub detonated: bool,
}

#[derive(Component, Debug)]
pub struct GrenadeFuse {
	pub remaining: f32,
}

/// Authoritative detonation. VFX and blast both consume this.
#[derive(Message, Clone, Copy, Debug)]
pub struct GrenadeDetonated {
	pub position: Vec3,
	pub source: Entity,
	pub effect: GrenadeEffect,
	pub radius: f32,
	pub damage: f32,
}

/// Animated membership, contacts Fixed and other Animated (terrain and capsules).
pub fn grenade_layers() -> CollisionLayers {
	PhysicsInteractionLayer::animated_layers()
}

pub fn grenade_mesh() -> Mesh {
	Sphere::new(0.5).mesh().ico(3).expect("grenade sphere")
}

/// Late-drop shape: most of the peak holds until the outer third, then zero at `radius`.
pub const BLAST_FALLOFF_K: f32 = 3.0;

/// Full damage at the origin, zero at `radius`.
pub fn blast_amount(damage: f32, radius: f32, distance: f32) -> f32 {
	if damage <= 0.0 || radius <= 1e-4 || distance > radius {
		return 0.0;
	}
	let t = (distance / radius).clamp(0.0, 1.0);
	let denom = 1.0 - (-BLAST_FALLOFF_K).exp();
	if denom <= 1e-6 {
		return damage * (1.0 - t);
	}
	damage * (1.0 - (-BLAST_FALLOFF_K * (1.0 - t)).exp()) / denom
}

pub fn spawn_thrown_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<GrenadeMaterial>,
	origin: Vec3,
	velocity: Vec3,
	spec: GrenadeSpec,
	stats: GrenadeStats,
	effect: GrenadeEffect,
	source: Entity,
) -> Entity {
	let radius = stats.radius.max(0.08);
	let collider = Collider::sphere(radius);
	let volume = (4.0 / 3.0) * PI * radius * radius * radius;
	let density = (stats.mass / volume.max(1e-6)).max(0.05);
	let tumble = velocity.cross(Vec3::Y).normalize_or(Vec3::X) * 10.0;
	commands
		.spawn((
			(
				Name::new("thrown-grenade"),
				Transform {
					translation: origin,
					rotation: Quat::from_rotation_arc(Vec3::Y, velocity.normalize_or(Vec3::Y)),
					scale: Vec3::splat(radius * 2.0),
				},
				Visibility::default(),
				Mesh3d(meshes.add(grenade_mesh())),
				MeshMaterial3d(materials.add(grenade_material())),
				RigidBody::Dynamic,
				MassPropertiesBundle::from_shape(&collider, density),
				collider,
				grenade_layers(),
			),
			(
				LinearVelocity(velocity),
				AngularVelocity(tumble),
				// Unit gravity, no linear damping / max-speed cap: catalog launch_speed
				// is the live throw. A 30 m/s standing lob lands past 30 m.
				GravityScale(1.0),
				Restitution {
					coefficient: stats.restitution,
					combine_rule: CoefficientCombine::Average,
				},
				Friction {
					dynamic_coefficient: stats.friction,
					static_coefficient: stats.friction,
					combine_rule: CoefficientCombine::Average,
				},
				ThrownGrenade { spec, stats, effect, source, detonated: false },
				GrenadeFuse { remaining: stats.fuse },
				CollidingEntities::default(),
				sound::GrenadeBounceClock { next_at: 0.0, touching: Vec::new() },
			),
		))
		.id()
}

pub fn tick_grenade_fuses(
	time: Res<Time>,
	mut detonations: MessageWriter<GrenadeDetonated>,
	mut commands: Commands,
	mut grenades: Query<(Entity, &Transform, &mut ThrownGrenade, &mut GrenadeFuse)>,
) {
	let dt = time.delta_secs();
	for (entity, transform, mut grenade, mut fuse) in &mut grenades {
		if grenade.detonated {
			continue;
		}
		fuse.remaining -= dt;
		if fuse.remaining > 0.0 {
			continue;
		}
		grenade.detonated = true;
		detonations.write(GrenadeDetonated {
			position: transform.translation,
			source: grenade.source,
			effect: grenade.effect,
			radius: grenade.stats.blast_radius,
			damage: grenade.stats.blast_damage,
		});
		commands.entity(entity).try_despawn();
	}
}

/// Animated colliders only; terrain is [`PhysicsInteractionLayer::Fixed`].
/// The thrower stays in the overlap so a short fuse can injure them.
fn blast_overlap_filter() -> SpatialQueryFilter {
	PhysicsInteractionLayer::Animated.query_filter()
}

/// Extension point for wall occlusion before falloff. Always clear until LOS is wired.
fn blast_line_of_sight_clear(
	_origin: Vec3,
	_target: Vec3,
	_spatial: &SpatialQuery,
	_filter: &SpatialQueryFilter,
) -> bool {
	// Follow-up: ray from origin toward target against Fixed (see spotting `clear_segment`).
	true
}

fn blast_target_point(
	target: Entity,
	collider: Entity,
	transforms: &Query<(Entity, &Transform, Option<&GlobalTransform>)>,
) -> Option<Vec3> {
	for entity in [target, collider] {
		if let Ok((_, transform, global)) = transforms.get(entity) {
			return Some(global.map(GlobalTransform::translation).unwrap_or(transform.translation));
		}
	}
	None
}

/// Resolve spatial overlap hits to [`Hit`]s with radial falloff. Unit-tested without Avian overlap.
fn collect_blast_hits(
	blast: &GrenadeDetonated,
	candidates: &[Entity],
	transforms: &Query<(Entity, &Transform, Option<&GlobalTransform>)>,
	health: &Query<(), With<damage::Health>>,
	parents: &Query<&ChildOf>,
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
) -> Vec<Hit> {
	let mut damaged = Vec::new();
	let mut out = Vec::new();
	for &collider in candidates {
		let parent = parents.get(collider).ok().map(|child| child.parent());
		let target = damage::Health::entity_or_parent(
			collider,
			parent,
			health.contains(collider),
			parent.is_some_and(|parent| health.contains(parent)),
		);
		if !health.contains(target) || damaged.contains(&target) {
			continue;
		}
		let Some(point) = blast_target_point(target, collider, transforms) else {
			continue;
		};
		if !blast_line_of_sight_clear(blast.position, point, spatial, filter) {
			continue;
		}
		let amount = blast_amount(blast.damage, blast.radius, point.distance(blast.position));
		if amount <= 1e-3 {
			continue;
		}
		damaged.push(target);
		out.push(Hit {
			target,
			source: Some(blast.source),
			amount,
			point,
			allow_self: target == blast.source,
		});
	}
	out
}

/// Sphere overlap for blast candidates, then radial falloff and [`Hit`] writes. `source` is
/// the thrower so player-thrown blasts feed hit markers.
pub fn apply_grenade_blasts(
	mut detonations: MessageReader<GrenadeDetonated>,
	spatial: SpatialQuery,
	transforms: Query<(Entity, &Transform, Option<&GlobalTransform>)>,
	health: Query<(), With<damage::Health>>,
	parents: Query<&ChildOf>,
	mut hits: MessageWriter<Hit>,
) {
	for blast in detonations.read() {
		let filter = blast_overlap_filter();
		let candidates = spatial.shape_intersections(
			&Collider::sphere(blast.radius),
			blast.position,
			Quat::IDENTITY,
			&filter,
		);
		for hit in collect_blast_hits(
			&blast,
			&candidates,
			&transforms,
			&health,
			&parents,
			&spatial,
			&filter,
		) {
			hits.write(hit);
		}
	}
}

pub struct GrenadesPlugin;

impl Plugin for GrenadesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
			app.add_plugins(GrenadeMaterialPlugin);
		}
		sound::configure_grenade_audio(app);
		app.add_message::<GrenadeDetonated>().add_message::<Hit>().add_systems(
			Update,
			(tick_grenade_fuses, apply_grenade_blasts.after(tick_grenade_fuses)),
		);
	}
}

#[cfg(test)]
mod tests {
	use bevy::asset::AssetPlugin;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::mesh::MeshPlugin;
	use damage::Health;

	use super::*;

	fn standard_blast() -> GrenadeDetonated {
		let stats = GrenadeStats::standard();
		GrenadeDetonated {
			position: Vec3::ZERO,
			source: Entity::from_bits(1),
			effect: GrenadeEffect::from_stats(&stats),
			radius: stats.blast_radius,
			damage: stats.blast_damage,
		}
	}

	fn blast_test_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			AssetPlugin::default(),
			MeshPlugin,
		));
		app.finish();
		app.init_resource::<Messages<GrenadeDetonated>>();
		app.init_resource::<Messages<Hit>>();
		app.update();
		app
	}

	fn spawn_damageable(app: &mut App, translation: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(translation),
				RigidBody::Dynamic,
				Collider::capsule(0.4, 1.0),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id()
	}

	fn run_blast_system(app: &mut App, blast: GrenadeDetonated) {
		app.world_mut().resource_mut::<Messages<GrenadeDetonated>>().write(blast);
		app.update();
		app.world_mut().run_system_once(apply_grenade_blasts).expect("blast");
	}

	fn collect_hits(
		world: &mut World,
		blast: GrenadeDetonated,
		candidates: Vec<Entity>,
	) -> Vec<Hit> {
		world
			.run_system_once(
				move |spatial: SpatialQuery,
				      transforms: Query<(Entity, &Transform, Option<&GlobalTransform>)>,
				      health: Query<(), With<Health>>,
				      parents: Query<&ChildOf>| {
					let filter = blast_overlap_filter();
					collect_blast_hits(
						&blast,
						&candidates,
						&transforms,
						&health,
						&parents,
						&spatial,
						&filter,
					)
				},
			)
			.expect("collect")
	}

	#[test]
	fn effect_from_stats_forwards_scale() {
		let stats = GrenadeStats::standard();
		let effect = GrenadeEffect::from_stats(&stats);
		assert!((effect.scale - stats.effect_scale).abs() < 1e-4);
	}

	#[test]
	fn fuse_expires_once() {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Messages<GrenadeDetonated>>();
		let stats = GrenadeStats::standard();
		let id = world
			.spawn((
				Transform::from_xyz(1.0, 0.5, 2.0),
				ThrownGrenade {
					spec: GrenadeSpec::standard(),
					stats,
					effect: GrenadeEffect::from_stats(&stats),
					source: Entity::PLACEHOLDER,
					detonated: false,
				},
				GrenadeFuse { remaining: 0.0 },
			))
			.id();
		world.run_system_once(tick_grenade_fuses).expect("tick");
		assert!(!world.entities().contains(id));
		assert_eq!(world.resource_mut::<Messages<GrenadeDetonated>>().drain().count(), 1);
	}

	#[test]
	fn thrown_body_contacts_terrain_and_characters() {
		let grenade = grenade_layers();
		let fixed = PhysicsInteractionLayer::fixed_layers();
		let animated = PhysicsInteractionLayer::animated_layers();
		assert!(grenade.interacts_with(fixed));
		assert!(fixed.interacts_with(grenade));
		assert!(grenade.interacts_with(animated));
	}

	#[test]
	fn blast_falls_off_to_the_rim() {
		assert!((blast_amount(80.0, 8.0, 0.0) - 80.0).abs() < 1e-4);
		let mid = blast_amount(80.0, 8.0, 4.0);
		assert!(mid > 60.0, "inner half should keep most of the peak, got {mid}");
		assert!(mid < 80.0);
		let outer = blast_amount(80.0, 8.0, 6.4);
		assert!(outer > 30.0, "outer third should still chunk, got {outer}");
		assert!(outer < mid);
		assert_eq!(blast_amount(80.0, 8.0, 8.1), 0.0);
	}

	#[test]
	fn collect_blast_hits_respects_radius_and_includes_the_thrower() {
		let mut app = blast_test_app();
		let in_range = spawn_damageable(&mut app, Vec3::new(2.0, 0.0, 0.0));
		let out_of_range = spawn_damageable(&mut app, Vec3::new(20.0, 0.0, 0.0));
		let thrower = spawn_damageable(&mut app, Vec3::new(0.5, 0.0, 0.0));
		app.update();
		let mut blast = standard_blast();
		blast.source = thrower;
		let hits = collect_hits(app.world_mut(), blast, vec![in_range, out_of_range, thrower]);
		assert_eq!(hits.len(), 2);
		assert!(hits.iter().any(|hit| hit.target == in_range && !hit.allow_self));
		assert!(hits.iter().any(|hit| hit.target == thrower && hit.allow_self));
		assert!(hits.iter().all(|hit| hit.source == Some(thrower) && hit.amount > 0.0));
	}

	#[test]
	fn blast_hits_carry_the_thrower() {
		let mut app = blast_test_app();
		let thrower = app.world_mut().spawn_empty().id();
		let target = spawn_damageable(&mut app, Vec3::new(2.0, 0.0, 0.0));
		let mut blast = standard_blast();
		blast.source = thrower;
		blast.position = Vec3::ZERO;
		run_blast_system(&mut app, blast);
		let hits: Vec<Hit> = app.world_mut().resource_mut::<Messages<Hit>>().drain().collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].target, target);
		assert_eq!(hits[0].source, Some(thrower));
		assert!(hits[0].amount > 0.0);
	}

	#[test]
	fn blast_skips_out_of_radius_targets() {
		let mut app = blast_test_app();
		let thrower = app.world_mut().spawn_empty().id();
		spawn_damageable(&mut app, Vec3::new(20.0, 0.0, 0.0));
		let mut blast = standard_blast();
		blast.source = thrower;
		run_blast_system(&mut app, blast);
		assert_eq!(app.world_mut().resource_mut::<Messages<Hit>>().drain().count(), 0);
	}

	#[test]
	fn blast_hits_the_thrower() {
		let mut app = blast_test_app();
		let thrower = spawn_damageable(&mut app, Vec3::new(0.5, 0.0, 0.0));
		let mut blast = standard_blast();
		blast.source = thrower;
		run_blast_system(&mut app, blast);
		let hits: Vec<Hit> = app.world_mut().resource_mut::<Messages<Hit>>().drain().collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].target, thrower);
		assert_eq!(hits[0].source, Some(thrower));
		assert!(hits[0].allow_self);
		assert!(hits[0].amount > 0.0);
	}
}
