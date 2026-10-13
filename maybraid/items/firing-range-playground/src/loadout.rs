//! Roll a generated clothing + firearm identity for a firing-range combatant.

use character_items::{
	realize_firearm_stats, CharacterSheet, FirearmMesh, FirearmSpec, FirearmStats, Inventory,
	InventoryItem, ItemRng, STARTER_CLOTHING_COUNT,
};
use characters::species::braidman::BraidmanConfig;
use firearm_user::kit_from_spec;
use firearms::FirearmKit;

/// Clothing recipe plus the combat kit assembled from a rolled firearm spec.
#[derive(Clone, Debug)]
pub(crate) struct CombatantLoadout {
	pub appearance: BraidmanConfig,
	pub spec: FirearmSpec,
	pub kit: FirearmKit,
	pub stats: FirearmStats,
	pub sheet: CharacterSheet,
}

pub(crate) fn roll_combatant(rng: &mut ItemRng) -> CombatantLoadout {
	let mut items = rng.random_starter_clothing(STARTER_CLOTHING_COUNT);
	let body = *rng.choose(FirearmMesh::VALUES).unwrap_or(&FirearmMesh::Bullpup);
	let spec = FirearmSpec::roll(rng, body);
	let stats = realize_firearm_stats(rng, &spec);
	items.push(InventoryItem::Firearm { spec, stats });
	let inventory = Inventory::with_starter_outfit(items);
	CombatantLoadout {
		appearance: appearance_from_inventory(&inventory),
		spec,
		kit: kit_from_spec(spec),
		stats,
		sheet: CharacterSheet::from_inventory(&inventory),
	}
}

fn appearance_from_inventory(inventory: &Inventory) -> BraidmanConfig {
	let mut config = BraidmanConfig::default_preview();
	for item in inventory.worn_items() {
		let InventoryItem::Clothing { mesh, material, .. } = item else {
			continue;
		};
		if !config.clothing.contains(mesh) {
			config.clothing.push(*mesh);
		}
		config.colors.set_clothing_color(*mesh, material.color);
		config.colors.set_clothing_material(*mesh, material.id);
	}
	config
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmSight, ItemColor, ProjectileKind};
	use firearms::{BarrelMesh, BodyMesh, GripMesh, SightMesh, TriggerBoxMesh};
	use std::collections::BTreeSet;

	#[test]
	fn starter_loadout_wears_clothes_and_queues_a_gun() {
		let loadout = roll_combatant(&mut ItemRng::from_seed(11));
		assert!(!loadout.appearance.clothing.is_empty());
		assert!(loadout.stats.damage > 0);
		assert!(loadout.sheet.health >= 1);
	}

	#[test]
	fn kit_mapping_preserves_bullpup_concept_slots() {
		let kit = kit_from_spec(FirearmSpec::from_mesh(FirearmMesh::Bullpup));
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.barrel, BarrelMesh::Bullpup);
		assert_eq!(kit.grip, GripMesh::BumpHandle);
	}

	#[test]
	fn kit_mapping_preserves_holorand_sight() {
		let mut spec = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		spec.kit.sight = FirearmSight::Holorand;
		let kit = kit_from_spec(spec);
		assert_eq!(kit.sight, SightMesh::Holorand);
	}

	#[test]
	fn kit_mapping_preserves_reltor_trigger_box() {
		let kit = kit_from_spec(FirearmSpec::from_mesh(FirearmMesh::Reltor));
		assert_eq!(kit.body, BodyMesh::Reltor);
		assert_eq!(kit.trigger_box, TriggerBoxMesh::Reltor);
	}

	#[test]
	fn gallery_style_rolls_are_visually_and_ballistically_diverse() {
		let mut rng = ItemRng::from_seed(7);
		let mut colors = BTreeSet::new();
		let mut kinds = BTreeSet::new();
		let mut greens = 0usize;
		let mut lasers = 0usize;
		const N: usize = 24;
		for _ in 0..N {
			let loadout = roll_combatant(&mut rng);
			colors.insert(loadout.spec.looks.body.color.label());
			kinds.insert(loadout.stats.projectile.label());
			if loadout.spec.looks.body.color == ItemColor::Green {
				greens += 1;
			}
			if loadout.stats.projectile == ProjectileKind::Laser {
				lasers += 1;
			}
		}
		assert!(colors.len() >= 4, "body colors {colors:?}");
		assert!(kinds.len() >= 2, "projectiles {kinds:?}");
		assert!(greens < N / 2, "green {greens}/{N}");
		assert!(lasers < (N * 3) / 4, "lasers {lasers}/{N}");
	}
}
