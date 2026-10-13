//! World-owned inventory cycle category and optional combat-hud mirror ([#1112](https://github.com/ramate-io/maybraid/issues/1112)).
//!
//! **Input model (this PR / #1112):** D-pad left / down / right selects the category;
//! **Y** and **[** / **]** cycle the equipped queue in that category.
//!
//! Draft [#1223](https://github.com/ramate-io/maybraid/pull/1223) proposed rebinding **Y** to
//! `CycleInventoryTarget` (rotate HUD cycle focus). That conflicts with #1112 acceptance
//! criteria; this lane keeps #1112 bindings and exposes [`InventoryCycleTarget`] plus HUD sync
//! so #1223 can layer teal panel chrome on the same world state.

use bevy::prelude::*;
use combat_hud::CombatHudInventoryCycleCategory;
use maybraid_character_controller::InventoryCyclingCategory;

use crate::inventory_cycling::ActiveInventoryCyclingCategory;

/// Same semantics as [`InventoryCyclingCategory`]; world/HUD-facing name for cross-lane sync.
pub type InventoryCycleTarget = InventoryCyclingCategory;

fn to_hud(category: InventoryCyclingCategory) -> CombatHudInventoryCycleCategory {
	match category {
		InventoryCyclingCategory::Weapons => CombatHudInventoryCycleCategory::Weapons,
		InventoryCyclingCategory::SkillMaps => CombatHudInventoryCycleCategory::SkillMaps,
		InventoryCyclingCategory::Lubikans => CombatHudInventoryCycleCategory::Lubikans,
	}
}

/// HUD-only snapshot. Missing `combat-hud` plugin is a no-op.
pub(crate) fn sync_combat_hud_cycle_category(
	category: Res<ActiveInventoryCyclingCategory>,
	hud: Option<ResMut<CombatHudInventoryCycleCategory>>,
) {
	let Some(mut hud) = hud else {
		return;
	};
	let next = to_hud(category.0);
	if *hud != next {
		*hud = next;
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(
		Update,
		sync_combat_hud_cycle_category.after(crate::inventory_cycling::InventoryCyclingCategorySystems),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn syncs_combat_hud_when_present() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::SkillMaps));
		world.insert_resource(CombatHudInventoryCycleCategory::Weapons);
		world
			.run_system_once(sync_combat_hud_cycle_category)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			*world.resource::<CombatHudInventoryCycleCategory>(),
			CombatHudInventoryCycleCategory::SkillMaps
		);
		Ok(())
	}

	#[test]
	fn sync_is_noop_without_combat_hud_resource() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::Lubikans));
		world
			.run_system_once(sync_combat_hud_cycle_category)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}
}
