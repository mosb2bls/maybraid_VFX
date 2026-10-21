//! Leave-behind fireball beads: shrinking tailed balls along the flight.

use avian3d::prelude::{Collider, SpatialQuery, SpatialQueryFilter};
use damage::HitPayload;
use lod_avian::PhysicsInteractionLayer;
use projectiles::{Flight, ProjectileContact, ProjectileSource};

use bevy::prelude::*;

use crate::effects::{FIREBALL_DAMAGE, FIREBALL_RADIUS};
use crate::fireball_embers::FireballEffects;

const BEAD_SPACING: f32 = 1.55;
const BEAD_LIFE: f32 = 0.95;
const BEAD_START_SCALE: f32 = 0.82;
const BEAD_END_SCALE: f32 = 0.22;
const BEAD_DAMAGE_FRACTION: f32 = 0.22;

#[derive(Component)]
pub(crate) struct FireballTrail {
	last_drop: Vec3,
}

#[derive(Component)]
pub(crate) struct FireballBead {
	age: f32,
	max_age: f32,
	start_scale: f32,
	contacted: Vec<Entity>,
}

pub(crate) fn drop_fireball_beads(
	mut commands: Commands,
	effects: Option<Res<FireballEffects>>,
	mut heads: Query<(&Transform, &mut FireballTrail, &ProjectileSource), With<Flight>>,
) {
	let Some(effects) = effects else {
		return;
	};
	for (transform, mut trail, source) in &mut heads {
		let pos = transform.translation;
		if !trail.last_drop.is_finite() {
			trail.last_drop = pos;
			continue;
		}
		if pos.distance(trail.last_drop) < BEAD_SPACING {
			continue;
		}
		trail.last_drop = pos;
		spawn_bead(&mut commands, &effects, *transform, source.0);
	}
}

fn spawn_bead(
	commands: &mut Commands,
	effects: &FireballEffects,
	transform: Transform,
	source: Entity,
) {
	let scale = BEAD_START_SCALE;
	commands.spawn((
		Name::new("fireball-bead"),
		FireballBead { age: 0.0, max_age: BEAD_LIFE, start_scale: scale, contacted: Vec::new() },
		Mesh3d(effects.mesh.clone()),
		MeshMaterial3d(effects.material.clone()),
		Transform { scale: Vec3::splat(scale), ..transform },
		Visibility::default(),
		ProjectileSource(source),
		HitPayload { amount: FIREBALL_DAMAGE * BEAD_DAMAGE_FRACTION },
	));
}

pub(crate) fn tick_fireball_beads(
	time: Res<Time>,
	mut commands: Commands,
	mut beads: Query<(Entity, &mut FireballBead, &mut Transform)>,
) {
	let dt = time.delta_secs();
	for (entity, mut bead, mut transform) in &mut beads {
		bead.age = (bead.age + dt).min(bead.max_age);
		let life = bead.age / bead.max_age.max(1e-3);
		transform.scale = Vec3::splat(bead.start_scale.lerp(BEAD_END_SCALE, life));
		if bead.age >= bead.max_age {
			commands.entity(entity).despawn();
		}
	}
}

pub(crate) fn contact_fireball_beads(
	spatial: SpatialQuery,
	mut contacts: MessageWriter<ProjectileContact>,
	mut beads: Query<(Entity, &Transform, &mut FireballBead, &ProjectileSource)>,
) {
	for (entity, transform, mut bead, source) in &mut beads {
		let life = bead.age / bead.max_age.max(1e-3);
		let radius = FIREBALL_RADIUS * bead.start_scale.lerp(BEAD_END_SCALE, life);
		let filter = SpatialQueryFilter::from_mask([
			PhysicsInteractionLayer::Fixed,
			PhysicsInteractionLayer::Animated,
		])
		.with_excluded_entities([entity, source.0]);
		let hits = spatial.shape_intersections(
			&Collider::sphere(radius.max(0.08)),
			transform.translation,
			transform.rotation,
			&filter,
		);
		for target in hits {
			if target == entity || target == source.0 {
				continue;
			}
			if bead.contacted.contains(&target) {
				continue;
			}
			bead.contacted.push(target);
			contacts.write(ProjectileContact {
				projectile: entity,
				source: Some(source.0),
				target,
				point: transform.translation,
				normal: Vec3::Y,
			});
		}
	}
}

pub(crate) fn dress_trail(commands: &mut Commands, projectile: Entity, origin: Vec3) {
	commands.entity(projectile).insert(FireballTrail { last_drop: origin });
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn beads_shrink_and_deal_a_fraction() {
		assert!(BEAD_END_SCALE < BEAD_START_SCALE);
		assert!(BEAD_SPACING > FIREBALL_RADIUS);
		assert!((FIREBALL_DAMAGE * BEAD_DAMAGE_FRACTION - 7.7).abs() < 1e-5);
	}
}
