//! Query-only hit hulls. The motor stays on the vertical body capsule.

use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use characters::{CharacterRoot, HeadCapsule, HitCapsule, LocomotionCapsule};
use lod_avian::PhysicsInteractionLayer;

pub const HIT_VOLUME_NAME: &str = "hit-capsule";
pub const HEAD_VOLUME_NAME: &str = "head-capsule";

/// Marker on the query-only child collider.
#[derive(Component, Clone, Copy, Debug)]
pub struct HitVolume;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum HitVolumeKind {
	Body,
	Head,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HitVolumeOf(pub Entity);

pub(crate) fn apply_hit_capsule(commands: &mut Commands, body: Entity, hull: LocomotionCapsule) {
	match hull.hit_capsule() {
		Some(hit) => {
			commands.entity(body).insert(hit);
		}
		None => {
			commands.entity(body).remove::<HitCapsule>();
		}
	}
	match hull.head_capsule() {
		Some(head) => {
			commands.entity(body).insert(head);
		}
		None => {
			commands.entity(body).remove::<HeadCapsule>();
		}
	}
}

/// Spawn or refresh one sensor child per query hull. Idempotent.
pub(crate) fn maintain_hit_volumes(
	bodies: Query<
		(Entity, Option<&HitCapsule>, Option<&HeadCapsule>),
		Or<(With<HitCapsule>, With<HeadCapsule>)>,
	>,
	changed_hit: Query<Entity, Changed<HitCapsule>>,
	changed_head: Query<Entity, Changed<HeadCapsule>>,
	volumes: Query<(Entity, &HitVolumeOf, &HitVolumeKind)>,
	mut removed_hit: RemovedComponents<HitCapsule>,
	mut removed_head: RemovedComponents<HeadCapsule>,
	mut commands: Commands,
) {
	let mut by_key: HashMap<(Entity, HitVolumeKind), Vec<Entity>> = HashMap::new();
	for (volume, of, kind) in &volumes {
		by_key.entry((of.0, *kind)).or_default().push(volume);
	}

	for body in removed_hit.read() {
		despawn_kind(&mut by_key, body, HitVolumeKind::Body, &mut commands);
	}
	for body in removed_head.read() {
		despawn_kind(&mut by_key, body, HitVolumeKind::Head, &mut commands);
	}

	for (body, hit, head) in &bodies {
		if let Some(hit) = hit {
			maintain_one(
				&mut by_key,
				body,
				HitVolumeKind::Body,
				HIT_VOLUME_NAME,
				hit.local_transform(),
				Collider::capsule(hit.radius, hit.length),
				changed_hit.get(body).is_ok(),
				&mut commands,
			);
		} else {
			despawn_kind(&mut by_key, body, HitVolumeKind::Body, &mut commands);
		}
		if let Some(head) = head {
			maintain_one(
				&mut by_key,
				body,
				HitVolumeKind::Head,
				HEAD_VOLUME_NAME,
				head.local_transform(),
				head_collider(*head),
				changed_head.get(body).is_ok(),
				&mut commands,
			);
		} else {
			despawn_kind(&mut by_key, body, HitVolumeKind::Head, &mut commands);
		}
	}

	for extras in by_key.into_values() {
		for volume in extras {
			commands.entity(volume).despawn();
		}
	}
}

fn head_collider(head: HeadCapsule) -> Collider {
	Collider::capsule(head.radius, head.length.max(0.0))
}

fn despawn_kind(
	by_key: &mut HashMap<(Entity, HitVolumeKind), Vec<Entity>>,
	body: Entity,
	kind: HitVolumeKind,
	commands: &mut Commands,
) {
	if let Some(existing) = by_key.remove(&(body, kind)) {
		for volume in existing {
			commands.entity(volume).despawn();
		}
	}
}

fn maintain_one(
	by_key: &mut HashMap<(Entity, HitVolumeKind), Vec<Entity>>,
	body: Entity,
	kind: HitVolumeKind,
	name: &'static str,
	transform: Transform,
	collider: Collider,
	changed: bool,
	commands: &mut Commands,
) {
	let existing = by_key.remove(&(body, kind)).unwrap_or_default();
	match existing.split_first() {
		Some((&volume, extras)) => {
			for extra in extras {
				commands.entity(*extra).despawn();
			}
			if changed {
				commands.entity(volume).insert((collider, transform));
			}
		}
		None => {
			commands.spawn((
				Name::new(name),
				HitVolume,
				kind,
				HitVolumeOf(body),
				ChildOf(body),
				transform,
				collider,
				Sensor,
				PhysicsInteractionLayer::animated_layers(),
			));
		}
	}
}

/// Body rotation is locked; facing lives on the visual. Follow that yaw.
pub(crate) fn align_hit_volumes(
	visuals: Query<(&Transform, &ChildOf), With<CharacterRoot>>,
	hulls: Query<
		(Option<&HitCapsule>, Option<&HeadCapsule>),
		Or<(With<HitCapsule>, With<HeadCapsule>)>,
	>,
	mut volumes: Query<(&HitVolumeOf, &HitVolumeKind, &mut Transform), Without<CharacterRoot>>,
) {
	let mut facing: HashMap<Entity, Quat> = HashMap::new();
	for (transform, child) in &visuals {
		facing.entry(child.parent()).or_insert(transform.rotation);
	}
	for (of, kind, mut transform) in &mut volumes {
		let Ok((hit, head)) = hulls.get(of.0) else {
			continue;
		};
		let rest = match kind {
			HitVolumeKind::Body => hit.map(|hit| hit.local_transform()),
			HitVolumeKind::Head => head.map(|head| head.local_transform()),
		};
		let Some(rest) = rest else {
			continue;
		};
		let yaw = facing.get(&of.0).copied().unwrap_or(Quat::IDENTITY);
		transform.rotation = yaw * rest.rotation;
		transform.translation = yaw * rest.translation;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::apply_character_controller;
	use anyhow::{anyhow, Result};
	use bevy::ecs::system::RunSystemOnce;

	fn volume_count(world: &mut World, body: Entity) -> usize {
		let mut volumes = world.query::<(&HitVolume, &ChildOf, &Sensor)>();
		volumes.iter(world).filter(|(_, child, _)| child.parent() == body).count()
	}

	fn kind_count(world: &mut World, body: Entity, kind: HitVolumeKind) -> usize {
		let mut volumes = world.query::<(&HitVolume, &HitVolumeOf, &HitVolumeKind)>();
		volumes.iter(world).filter(|(_, of, k)| of.0 == body && **k == kind).count()
	}

	#[test]
	fn quadruped_controller_gets_one_sensor_child() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, (maintain_hit_volumes, align_hit_volumes).chain());
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::QUADRUPED);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		app.update();
		if volume_count(app.world_mut(), body) != 1 {
			return Err(anyhow!("quadruped should stamp one hit-capsule child"));
		}
		if app.world().get::<HitCapsule>(body).is_none() {
			return Err(anyhow!("pronograde hull should keep HitCapsule on the body"));
		}
		Ok(())
	}

	#[test]
	fn humanoid_controller_has_no_hit_volume() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, maintain_hit_volumes);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		if app.world().get::<HitCapsule>(body).is_some() {
			return Err(anyhow!("humanoid must not carry a hit hull"));
		}
		if app.world().get::<HeadCapsule>(body).is_some() {
			return Err(anyhow!("stock humanoid must not carry a head hull"));
		}
		if volume_count(app.world_mut(), body) != 0 {
			return Err(anyhow!("humanoid must not spawn a hit-capsule child"));
		}
		Ok(())
	}

	#[test]
	fn oversized_head_gets_one_head_sensor_child() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, maintain_hit_volumes);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(
					&mut commands,
					body,
					LocomotionCapsule::HUMANOID.with_head_scale(2.0),
				);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		if app.world().get::<HeadCapsule>(body).is_none() {
			return Err(anyhow!("oversized head should keep HeadCapsule on the body"));
		}
		if kind_count(app.world_mut(), body, HitVolumeKind::Head) != 1 {
			return Err(anyhow!("oversized head should stamp one head-capsule child"));
		}
		if kind_count(app.world_mut(), body, HitVolumeKind::Body) != 0 {
			return Err(anyhow!("biped must not spawn a pronograde hit-capsule"));
		}
		Ok(())
	}

	#[test]
	fn maintain_is_idempotent() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, maintain_hit_volumes);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::QUADRUPED);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		app.update();
		app.update();
		if volume_count(app.world_mut(), body) != 1 {
			return Err(anyhow!("re-running maintain must not spawn a second child"));
		}
		Ok(())
	}

	#[test]
	fn hit_volume_follows_visual_yaw() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, (maintain_hit_volumes, align_hit_volumes).chain());
		let yaw = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::QUADRUPED);
				commands.spawn((CharacterRoot, ChildOf(body), Transform::IDENTITY));
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		let visual = {
			let mut roots = app.world_mut().query_filtered::<Entity, With<CharacterRoot>>();
			roots.iter(app.world()).next()
		};
		let Some(visual) = visual else {
			return Err(anyhow!("missing CharacterRoot visual"));
		};
		let Some(mut visual_transform) = app.world_mut().get_mut::<Transform>(visual) else {
			return Err(anyhow!("visual has no Transform"));
		};
		*visual_transform = Transform::from_rotation(yaw);
		app.update();
		let Some(hit) = app.world().get::<HitCapsule>(body).copied() else {
			return Err(anyhow!("missing HitCapsule"));
		};
		let rest = hit.local_transform();
		let aligned = {
			let mut volumes = app.world_mut().query::<(&HitVolume, &Transform)>();
			volumes.iter(app.world()).next().map(|(_, transform)| *transform)
		};
		let Some(aligned) = aligned else {
			return Err(anyhow!("missing hit volume transform"));
		};
		let expected_rotation = yaw * rest.rotation;
		let got_axis = aligned.rotation * Vec3::Y;
		let want_axis = expected_rotation * Vec3::Y;
		if (got_axis - want_axis).length() > 1e-3 {
			return Err(anyhow!(
				"hit hull should follow visual yaw, axis {got_axis} want {want_axis}"
			));
		}
		if (aligned.translation - yaw * rest.translation).length() > 1e-4 {
			return Err(anyhow!("hit hull origin should yaw with the visual"));
		}
		Ok(())
	}
}
