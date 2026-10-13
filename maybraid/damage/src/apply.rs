use bevy::prelude::*;

use crate::{DamageApplied, Died, HeadshotBand, Health, Hit};

pub fn apply_hits(
	mut hits: MessageReader<Hit>,
	mut health: Query<&mut Health>,
	parents: Query<&ChildOf>,
	bands: Query<(&GlobalTransform, &HeadshotBand)>,
	mut applied: MessageWriter<DamageApplied>,
	mut died: MessageWriter<Died>,
) {
	for hit in hits.read() {
		let parent = parents.get(hit.target).ok().map(|child| child.parent());
		let target = Health::entity_or_parent(
			hit.target,
			parent,
			health.contains(hit.target),
			parent.is_some_and(|parent| health.contains(parent)),
		);
		if hit.source == Some(target) && !hit.allow_self {
			continue;
		}
		let Ok(mut pool) = health.get_mut(target) else {
			continue;
		};
		if pool.is_dead() {
			continue;
		}
		let amount = bands
			.get(target)
			.map(|(transform, band)| band.scale(transform, hit.point, hit.amount))
			.unwrap_or(hit.amount);
		let lost = pool.take_damage(amount);
		applied.write(DamageApplied {
			target,
			source: hit.source,
			amount,
			applied: lost,
			remaining: pool.current,
			point: hit.point,
		});
		if pool.is_dead() {
			died.write(Died { entity: target, source: hit.source, point: hit.point });
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};

	#[test]
	fn remounts_child_collider_onto_parent_health() -> Result<()> {
		let mut app = App::new();
		app.add_message::<Hit>().add_message::<DamageApplied>().add_message::<Died>();
		app.add_systems(Update, apply_hits);
		let parent = app.world_mut().spawn(Health::from_max(100.0)).id();
		let child = app.world_mut().spawn(ChildOf(parent)).id();
		app.world_mut().write_message(Hit {
			target: child,
			source: None,
			amount: 25.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();
		let remaining = app.world().get::<Health>(parent).map(|health| health.current);
		if remaining != Some(75.0) {
			return Err(anyhow!("parent health should drop, got {remaining:?}"));
		}
		Ok(())
	}

	#[test]
	fn ignores_self_hit_on_own_hit_volume() -> Result<()> {
		let mut app = App::new();
		app.add_message::<Hit>().add_message::<DamageApplied>().add_message::<Died>();
		app.add_systems(Update, apply_hits);
		let parent = app.world_mut().spawn(Health::from_max(100.0)).id();
		let child = app.world_mut().spawn(ChildOf(parent)).id();
		app.world_mut().write_message(Hit {
			target: child,
			source: Some(parent),
			amount: 25.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();
		let remaining = app.world().get::<Health>(parent).map(|health| health.current);
		if remaining != Some(100.0) {
			return Err(anyhow!("self-hit on a child hull should not injure, got {remaining:?}"));
		}
		Ok(())
	}

	#[test]
	fn overkill_reports_applied_hp_not_requested() -> Result<()> {
		let mut app = App::new();
		app.add_message::<Hit>().add_message::<DamageApplied>().add_message::<Died>();
		app.add_systems(Update, apply_hits);
		let target = app.world_mut().spawn(Health::from_max(10.0)).id();
		app.world_mut().write_message(Hit {
			target,
			source: None,
			amount: 25.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();
		let remaining = app.world().get::<Health>(target).map(|health| health.current);
		if remaining != Some(0.0) {
			return Err(anyhow!("overkill should empty the pool, got {remaining:?}"));
		}
		Ok(())
	}

	#[test]
	fn already_dead_health_ignores_further_hits() -> Result<()> {
		let mut app = App::new();
		app.add_message::<Hit>().add_message::<DamageApplied>().add_message::<Died>();
		app.add_systems(Update, apply_hits);
		let target = app.world_mut().spawn(Health { current: 0.0, max: 100.0 }).id();
		app.world_mut().write_message(Hit {
			target,
			source: None,
			amount: 25.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();
		let remaining = app.world().get::<Health>(target).map(|health| health.current);
		if remaining != Some(0.0) {
			return Err(anyhow!("dead health should stay at zero, got {remaining:?}"));
		}
		let died: Vec<_> = app.world_mut().resource_mut::<Messages<Died>>().drain().collect();
		if !died.is_empty() {
			return Err(anyhow!("already-dead hits must not emit Died, got {}", died.len()));
		}
		Ok(())
	}

	#[test]
	fn allow_self_injures_the_source() -> Result<()> {
		let mut app = App::new();
		app.add_message::<Hit>().add_message::<DamageApplied>().add_message::<Died>();
		app.add_systems(Update, apply_hits);
		let thrower = app.world_mut().spawn(Health::from_max(100.0)).id();
		app.world_mut().write_message(Hit {
			target: thrower,
			source: Some(thrower),
			amount: 40.0,
			point: Vec3::ZERO,
			allow_self: true,
		});
		app.update();
		let remaining = app.world().get::<Health>(thrower).map(|health| health.current);
		if remaining != Some(60.0) {
			return Err(anyhow!("authored self-hit should injure, got {remaining:?}"));
		}
		Ok(())
	}
}
