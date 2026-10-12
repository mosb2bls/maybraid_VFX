use bevy::prelude::*;

use crate::CombatTargeting;

/// Ordering point for systems that consume ranked targets.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombatTargetingSystems {
	Rank,
}

/// Rebalances every [`CombatTargeting`] during [`Update`].
pub struct CombatTargetingPlugin;

impl Plugin for CombatTargetingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(Update, CombatTargetingSystems::Rank)
			.add_systems(Update, rank_combat_targets.in_set(CombatTargetingSystems::Rank));
	}
}

/// Runs continuously because contact expiry and influence decay are time-based.
pub fn rank_combat_targets(time: Res<Time>, mut combatants: Query<&mut CombatTargeting>) {
	let now = time.elapsed_secs();
	for mut targeting in &mut combatants {
		if !targeting.enabled {
			if !targeting.ranked.is_empty() || targeting.engaged.is_some() {
				targeting.ranked.clear();
				targeting.clear_engagement();
			}
			continue;
		}
		if targeting.needs_rebalance(now) {
			targeting.rebalance(now);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn disabled_idle_combatant_is_not_rewritten() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<Time>();
		let entity = world
			.spawn(CombatTargeting {
				enabled: false,
				dirty: false,
				next_rebalance_at: f32::INFINITY,
				..default()
			})
			.id();
		anyhow::ensure!(world.run_system_once(rank_combat_targets).is_ok());
		let before = world
			.entity(entity)
			.get_change_ticks::<CombatTargeting>()
			.ok_or_else(|| anyhow::anyhow!("missing targeting"))?
			.changed
			.get();
		anyhow::ensure!(world.run_system_once(rank_combat_targets).is_ok());
		let after = world
			.entity(entity)
			.get_change_ticks::<CombatTargeting>()
			.ok_or_else(|| anyhow::anyhow!("missing targeting"))?
			.changed
			.get();
		anyhow::ensure!(before == after);
		Ok(())
	}
}
