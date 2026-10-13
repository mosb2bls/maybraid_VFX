use bevy::prelude::Component;
use character_items::LootFraction;
use mob_characters::FromMobNumber;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MobKind {
	#[default]
	Herd,
	Pack,
	Raider,
	Guard,
	Pleb,
	Rambles,
	Brawler,
}

impl MobKind {
	pub const VALUES: [Self; 7] = [
		Self::Herd,
		Self::Pack,
		Self::Raider,
		Self::Guard,
		Self::Pleb,
		Self::Rambles,
		Self::Brawler,
	];

	pub const fn count_range(self) -> (usize, usize) {
		match self {
			Self::Herd => (1, 24),
			Self::Pack | Self::Raider | Self::Guard => (3, 12),
			Self::Pleb => (10, 24),
			Self::Rambles => (1, 12),
			Self::Brawler => (6, 12),
		}
	}

	/// Death loot kept from the bag. Combat families drop a fraction so
	/// corpses do not carpet the world; other families drop nothing.
	pub const fn loot_fraction(self) -> LootFraction {
		match self {
			Self::Raider | Self::Guard => LootFraction::ONE_THIRD,
			Self::Brawler => LootFraction::ONE_TWELFTH,
			Self::Herd | Self::Pack | Self::Pleb | Self::Rambles => LootFraction::NONE,
		}
	}
}

impl FromMobNumber for MobKind {
	fn from_num(num: f32) -> Self {
		let mixed = u64::from(num.to_bits()).wrapping_mul(0x9E37_79B9_7F4A_7C15);
		Self::VALUES[(mixed as usize) % Self::VALUES.len()]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn combat_families_drop_a_fraction_and_others_drop_nothing() {
		assert_eq!(MobKind::Raider.loot_fraction(), LootFraction::ONE_THIRD);
		assert_eq!(MobKind::Guard.loot_fraction(), LootFraction::ONE_THIRD);
		assert_eq!(MobKind::Brawler.loot_fraction(), LootFraction::ONE_TWELFTH);
		assert_eq!(MobKind::Pleb.loot_fraction(), LootFraction::NONE);
		assert_eq!(MobKind::Herd.loot_fraction(), LootFraction::NONE);
		assert_eq!(MobKind::Rambles.loot_fraction(), LootFraction::NONE);
	}
}
