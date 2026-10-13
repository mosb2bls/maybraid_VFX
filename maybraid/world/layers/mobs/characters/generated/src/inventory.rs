//! Inventory profiles that retain the real bag rather than a collapsed loadout.

use bevy::prelude::Component;
use character_items::{
	realize_firearm_stats, FirearmMesh, FirearmSpec, FirearmStats, Inventory, InventoryItem,
	ItemRng, STARTER_CLOTHING_COUNT,
};

use crate::number::{index, seed, FromMobNumber};

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterInventory {
	#[default]
	Empty,
	Clothed,
	Flashy,
	Grunt,
	Mercenary,
	Specialist,
	Mist,
}

impl CharacterInventory {
	pub const VALUES: [Self; 7] = [
		Self::Empty,
		Self::Clothed,
		Self::Flashy,
		Self::Grunt,
		Self::Mercenary,
		Self::Specialist,
		Self::Mist,
	];

	pub const fn carries_weapon(self) -> bool {
		matches!(self, Self::Grunt | Self::Mercenary | Self::Specialist | Self::Mist)
	}

	pub fn generate(self, num: f32) -> Inventory {
		if self == Self::Empty {
			return Inventory::default();
		}
		let mut rng = ItemRng::from_seed(seed(num, 0x1A6B_4A9D));
		let clothing_count = match self {
			Self::Clothed | Self::Grunt => STARTER_CLOTHING_COUNT,
			Self::Flashy | Self::Mercenary | Self::Specialist | Self::Mist => {
				STARTER_CLOTHING_COUNT.saturating_add(3)
			}
			Self::Empty => 0,
		};
		let mut items = rng.random_starter_clothing(clothing_count);
		if let Some(weapon) = self.weapon(num, &mut rng) {
			items.push(weapon);
		}
		Inventory::with_starter_outfit(items)
	}

	fn weapon(self, num: f32, rng: &mut ItemRng) -> Option<InventoryItem> {
		let (body, rolls) = match self {
			Self::Grunt => (FirearmMesh::Bullpup, 1),
			Self::Mercenary => (random_body(rng), 4),
			Self::Specialist => {
				let specialist =
					[FirearmMesh::Reltor, FirearmMesh::Samsonist, FirearmMesh::Snailer];
				(specialist[index(num, 0x5EEC_1A17, specialist.len())], 7)
			}
			Self::Mist => (random_body(rng), 16),
			Self::Empty | Self::Clothed | Self::Flashy => return None,
		};
		let mut best: Option<(FirearmSpec, FirearmStats)> = None;
		for _ in 0..rolls {
			let spec = FirearmSpec::roll(rng, body);
			let stats = realize_firearm_stats(rng, &spec);
			if best
				.as_ref()
				.is_none_or(|(_, current)| weapon_score(stats) > weapon_score(*current))
			{
				best = Some((spec, stats));
			}
		}
		best.map(|(spec, stats)| InventoryItem::Firearm { spec, stats })
	}
}

impl FromMobNumber for CharacterInventory {
	fn from_num(num: f32) -> Self {
		Self::VALUES[index(num, 0x1A6B_4A9D, Self::VALUES.len())]
	}
}

fn random_body(rng: &mut ItemRng) -> FirearmMesh {
	*rng.choose(FirearmMesh::VALUES).unwrap_or(&FirearmMesh::Bullpup)
}

fn weapon_score(stats: FirearmStats) -> u32 {
	u32::from(stats.damage)
		.saturating_mul(4)
		.saturating_add(u32::from(stats.penetration) / 20)
		.saturating_add(u32::from(stats.speed) / 10)
		.saturating_add(u32::from(stats.range))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn armed_profiles_materialize_a_selected_firearm() {
		for (index, profile) in
			[CharacterInventory::Grunt, CharacterInventory::Mercenary, CharacterInventory::Mist]
				.into_iter()
				.enumerate()
		{
			let inventory = profile.generate(index as f32 + 0.25);
			assert!(inventory.primary_weapon().is_some());
		}
	}

	#[test]
	fn empty_profile_has_no_items() {
		assert!(CharacterInventory::Empty.generate(0.5).items.is_empty());
	}
}
