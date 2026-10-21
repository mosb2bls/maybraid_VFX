//! World-owned inventory cycle category and optional combat-hud mirror ([#1112](https://github.com/ramate-io/maybraid/issues/1112)).
//!
//! **Input model:** D-pad left / down / right selects the category;
//! **Y** and **[** / **]** cycle the equipped queue in that category.
//! This module only mirrors that world state to the HUD and stamps cycle-panel
//! markers onto existing weapons / skill-map roots.

use bevy::prelude::*;
use combat_hud::{idle_cycle_outline, CombatHudCyclePanel, CombatHudInventoryCycleCategory};
use maybraid_character_controller::InventoryCyclingCategory;
use maybraid_skill_map::{SkillMapFirearmHud, SkillMapViewport};

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

/// Attach the generic cycle marker to existing weapons / skill-map panel roots.
pub(crate) fn stamp_cycle_panel_markers(
	mut commands: Commands,
	firearms: Query<Entity, (With<SkillMapFirearmHud>, Without<CombatHudCyclePanel>)>,
	maps: Query<Entity, (With<SkillMapViewport>, Without<CombatHudCyclePanel>)>,
) {
	for entity in &firearms {
		commands.entity(entity).insert((
			CombatHudCyclePanel(CombatHudInventoryCycleCategory::Weapons),
			idle_cycle_outline(),
		));
	}
	for entity in &maps {
		commands.entity(entity).insert((
			CombatHudCyclePanel(CombatHudInventoryCycleCategory::SkillMaps),
			idle_cycle_outline(),
		));
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(
		Update,
		(
			sync_combat_hud_cycle_category
				.after(maybraid_equipment::EquipmentSystems::SelectCategory),
			stamp_cycle_panel_markers,
		),
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

	#[test]
	fn y_does_not_change_the_synced_hud_category() -> anyhow::Result<()> {
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
		// SwapActive / Y cycles the equipped queue; the world category is unchanged.
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
	fn stamps_cycle_markers_on_existing_weapon_and_map_panels() -> anyhow::Result<()> {
		let mut world = World::new();
		let weapons = world.spawn(SkillMapFirearmHud).id();
		let maps = world.spawn(SkillMapViewport).id();
		world
			.run_system_once(stamp_cycle_panel_markers)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<CombatHudCyclePanel>(weapons).copied(),
			Some(CombatHudCyclePanel(CombatHudInventoryCycleCategory::Weapons))
		);
		assert_eq!(
			world.get::<CombatHudCyclePanel>(maps).copied(),
			Some(CombatHudCyclePanel(CombatHudInventoryCycleCategory::SkillMaps))
		);
		Ok(())
	}
}
