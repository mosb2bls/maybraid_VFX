//! D-pad inventory cycling category cue ([#1112](https://github.com/ramate-io/maybraid/issues/1112)).
//!
//! World owns the write (D-pad left / down / right selects the category).
//! This crate only reads it and draws a teal outline on the matching panel root.
//! **Y** cycles the equipped queue inside that category and does not move the outline.

use bevy::prelude::*;

/// Bright teal outline on the HUD panel the D-pad has selected.
pub const CYCLE_ACTIVE_TEAL: Color =
	Color::srgb(0x2E as f32 / 255.0, 0xF2 as f32 / 255.0, 0xE0 as f32 / 255.0);

pub const CYCLE_OUTLINE_WIDTH: f32 = 3.0;
pub const CYCLE_OUTLINE_OFFSET: f32 = 2.0;

/// Which equipped queue Y will cycle. Upstream inventory code owns updates.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombatHudInventoryCycleCategory {
	#[default]
	Weapons,
	SkillMaps,
	Lubikans,
}

impl CombatHudInventoryCycleCategory {
	pub const ALL: [Self; 3] = [Self::Weapons, Self::SkillMaps, Self::Lubikans];

	pub const fn is_lubikans(self) -> bool {
		matches!(self, Self::Lubikans)
	}
}

/// Marker on a HUD panel root. Any plate that should take the cycle outline
/// can carry this; the highlight system does not care which crate spawned it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatHudCyclePanel(pub CombatHudInventoryCycleCategory);

/// Idle outline so activating a panel does not change layout.
pub fn idle_cycle_outline() -> Outline {
	Outline::new(Val::Px(CYCLE_OUTLINE_WIDTH), Val::Px(CYCLE_OUTLINE_OFFSET), Color::NONE)
}

pub fn active_cycle_outline() -> Outline {
	Outline::new(Val::Px(CYCLE_OUTLINE_WIDTH), Val::Px(CYCLE_OUTLINE_OFFSET), CYCLE_ACTIVE_TEAL)
}

pub(crate) fn sync_cycle_panel_highlight(
	category: Option<Res<CombatHudInventoryCycleCategory>>,
	mut panels: Query<(&CombatHudCyclePanel, &mut Outline)>,
) {
	let active = category.map(|category| *category).unwrap_or_default();
	let live = active_cycle_outline();
	let idle = idle_cycle_outline();
	for (panel, mut outline) in &mut panels {
		let next = if panel.0 == active { live } else { idle };
		if *outline != next {
			*outline = next;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cycle_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<CombatHudInventoryCycleCategory>()
			.add_systems(PostUpdate, sync_cycle_panel_highlight);
		app
	}

	fn spawn_cycle_panels(world: &mut World) {
		for category in CombatHudInventoryCycleCategory::ALL {
			world.spawn((CombatHudCyclePanel(category), idle_cycle_outline()));
		}
	}

	fn live_categories(world: &mut World) -> Vec<CombatHudInventoryCycleCategory> {
		world
			.query::<(&CombatHudCyclePanel, &Outline)>()
			.iter(world)
			.filter(|(_, outline)| outline.color == CYCLE_ACTIVE_TEAL)
			.map(|(panel, _)| panel.0)
			.collect()
	}

	fn assert_only(world: &mut World, expected: CombatHudInventoryCycleCategory) {
		let live = live_categories(world);
		assert_eq!(live, vec![expected], "exactly one panel keeps the teal outline");
		let rows: Vec<_> = world
			.query::<(&CombatHudCyclePanel, &Outline)>()
			.iter(world)
			.map(|(panel, outline)| (panel.0, outline.color))
			.collect();
		assert_eq!(rows.len(), 3, "every cycle system must have a panel root");
		for (category, color) in rows {
			if category != expected {
				assert_eq!(color, Color::NONE, "{category:?} must restore the idle outline");
			}
		}
	}

	#[test]
	fn exactly_one_panel_has_the_teal_border() {
		let mut app = cycle_app();
		spawn_cycle_panels(app.world_mut());
		app.world_mut().insert_resource(CombatHudInventoryCycleCategory::Weapons);
		app.update();
		assert_only(app.world_mut(), CombatHudInventoryCycleCategory::Weapons);
	}

	#[test]
	fn teal_border_follows_dpad_category_selection() {
		let mut app = cycle_app();
		spawn_cycle_panels(app.world_mut());
		for expected in CombatHudInventoryCycleCategory::ALL {
			app.world_mut().insert_resource(expected);
			app.update();
			assert_only(app.world_mut(), expected);
		}
	}

	#[test]
	fn y_cycling_within_a_category_does_not_move_the_border() {
		let mut app = cycle_app();
		spawn_cycle_panels(app.world_mut());
		app.world_mut().insert_resource(CombatHudInventoryCycleCategory::SkillMaps);
		app.update();
		assert_only(app.world_mut(), CombatHudInventoryCycleCategory::SkillMaps);
		// Y / SwapActive cycles the equipped queue; the HUD category is unchanged.
		app.world_mut().insert_resource(CombatHudInventoryCycleCategory::SkillMaps);
		app.update();
		assert_only(app.world_mut(), CombatHudInventoryCycleCategory::SkillMaps);
	}
}
