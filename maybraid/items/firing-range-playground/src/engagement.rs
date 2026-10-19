//! Playground session bell: NPCs start on hold-fire and go weapons-free after the player shoots.

use bevy::prelude::*;
use firearm_intelligence::{FirearmEngagement, RulesOfEngagement};
use firearms::WeaponFired;
use player::{Npc, Player};

#[derive(Resource, Default)]
pub(crate) struct NpcEngagement {
	live: bool,
}

impl NpcEngagement {
	pub fn is_live(&self) -> bool {
		self.live
	}

	pub fn reset(&mut self) {
		self.live = false;
	}
}

/// Observe actual shots, rather than raw trigger input.
pub(crate) fn record_player_shot(
	players: Query<Entity, With<Player>>,
	mut fired: MessageReader<WeaponFired>,
	mut engagement: ResMut<NpcEngagement>,
	mut npcs: Query<&mut FirearmEngagement, With<Npc>>,
) {
	if !fired.read().any(|event| players.contains(event.shooter)) {
		return;
	}
	engagement.live = true;
	for mut rules in &mut npcs {
		rules.set_rules(RulesOfEngagement::WeaponsFree);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn ceasefire_stays_cold_until_a_player_shot() {
		let mut engagement = NpcEngagement::default();
		assert!(!engagement.is_live());
		engagement.live = true;
		assert!(engagement.is_live());
		engagement.reset();
		assert!(!engagement.is_live());
	}

	#[test]
	fn player_shot_releases_npc_weapons_free() -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		world.init_resource::<NpcEngagement>();
		world.init_resource::<Messages<WeaponFired>>();
		let player = world.spawn(Player).id();
		let npc = world.spawn((Npc, FirearmEngagement::hold())).id();
		world.write_message(WeaponFired::new(player, 0.0));
		world.run_system_once(record_player_shot)?;
		assert!(world.resource::<NpcEngagement>().is_live());
		assert_eq!(
			world.get::<FirearmEngagement>(npc).map(|rules| rules.rules),
			Some(RulesOfEngagement::WeaponsFree)
		);
		Ok(())
	}
}
