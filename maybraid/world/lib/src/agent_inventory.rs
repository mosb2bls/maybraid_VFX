//! Publish inventory-cycle snapshots for the agent. No menu or combat-hud types.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventoryItem};
use lubikans::{LubikanLedger, LubikanOwner};
use maybraid_agent::{AgentButton, InventoryObservation, PadIntent};
use maybraid_character_controller::InventoryCyclingCategory;
use world_player::Player as VegetationPlayer;

use crate::control::WorldGameplayEnabled;
use crate::inventory_cycling::{gameplay_cycling_allowed, ActiveInventoryCyclingCategory};
use crate::map_view::WorldMapView;
use game_commands::command::TextEntryFocus;

/// Last virtual-pad intent the agent applied this frame (for input-aware invariants).
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AgentLastPadIntent(pub PadIntent);

/// Bevy-free cycling snapshot filled from world state.
#[derive(Resource, Clone, Debug, Default)]
pub struct AgentInventoryState {
	pub observation: InventoryObservation,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_agent_inventory(
	mut state: ResMut<AgentInventoryState>,
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	category: Option<Res<ActiveInventoryCyclingCategory>>,
	players: Query<(&InventoryUser, Option<&LubikanLedger>), With<VegetationPlayer>>,
	bags: Query<&Inventory>,
	ledgers: Query<&LubikanLedger, With<LubikanOwner>>,
	intent: Option<Res<AgentLastPadIntent>>,
) {
	let mut observation = InventoryObservation {
		cycle_category: category
			.as_deref()
			.map(|category| category_script_id(category.0).into())
			.unwrap_or_default(),
		..InventoryObservation::default()
	};
	if let Some((user, player_ledger)) = players.iter().next() {
		if let Ok(bag) = bags.get(user.bag) {
			observation.weapons = queue_ids(bag, bag.weapons.iter().copied());
			observation.skill_maps = queue_ids(bag, bag.skills.iter().copied());
			observation.weapon = observation.weapons.first().cloned();
			observation.skill_map = observation.skill_maps.first().cloned();
		}
		let ledger = player_ledger.or_else(|| ledgers.iter().next());
		if let Some(ledger) = ledger {
			observation.lubikans = ledger
				.iter()
				.map(|account| lubikan_script_id(account.id().definition()))
				.collect();
			observation.lubikan =
				Some(lubikan_script_id(ledger.selection().resolved().definition()));
		}
	}

	let cycling_allowed = gameplay_cycling_allowed(&gameplay, &text_focus, map.as_deref());
	let last = state.observation.clone();
	let (pressed_y, pressed_category) = intent
		.as_deref()
		.map(|intent| cycle_buttons(&intent.0))
		.unwrap_or((false, false));
	observation.invariants =
		cycling_invariants(&last, &observation, pressed_y, pressed_category, cycling_allowed);
	state.observation = observation;
}

fn category_script_id(category: InventoryCyclingCategory) -> &'static str {
	match category {
		InventoryCyclingCategory::Weapons => "weapons",
		InventoryCyclingCategory::SkillMaps => "skill_maps",
		InventoryCyclingCategory::Lubikans => "lubikans",
	}
}

fn item_script_id(item: &InventoryItem) -> String {
	item.label().to_ascii_lowercase()
}

fn lubikan_script_id(definition: lubikans::LubikanDefinitionId) -> String {
	definition
		.well_known_label()
		.map(|label| label.to_ascii_lowercase())
		.unwrap_or_else(|| format!("lubikan-{}", definition.raw()))
}

fn queue_ids(bag: &Inventory, indices: impl Iterator<Item = usize>) -> Vec<String> {
	indices.filter_map(|index| bag.items.get(index).map(item_script_id)).collect()
}

fn cycle_buttons(intent: &PadIntent) -> (bool, bool) {
	let pressed_y = intent.buttons.contains(&AgentButton::Swap);
	let pressed_category = intent.buttons.iter().any(|button| {
		matches!(*button, AgentButton::DpadLeft | AgentButton::DpadDown | AgentButton::DpadRight)
	});
	(pressed_y, pressed_category)
}

/// Input-aware cycling invariants for playtests.
///
/// First-frame / empty-queue transitions are skipped so spawn and fixture load
/// do not look like a broken cycle.
pub(crate) fn cycling_invariants(
	last: &InventoryObservation,
	current: &InventoryObservation,
	pressed_y: bool,
	pressed_category: bool,
	cycling_allowed: bool,
) -> Vec<String> {
	let mut out = Vec::new();
	if last.cycle_category.is_empty() || current.cycle_category.is_empty() {
		return out;
	}
	if cycling_allowed
		&& pressed_y
		&& !pressed_category
		&& last.cycle_category != current.cycle_category
	{
		out.push(format!(
			"Y changed cycle category (was {} now {})",
			last.cycle_category, current.cycle_category
		));
	}
	if cycling_allowed && pressed_category && !pressed_y && current.equipped_changed(last) {
		out.push(format!(
			"D-pad changed active item while selecting {} (weapon {}→{} skill_map {}→{} lubikan {}→{})",
			current.cycle_category,
			last.weapon.as_deref().unwrap_or("null"),
			current.weapon.as_deref().unwrap_or("null"),
			last.skill_map.as_deref().unwrap_or("null"),
			current.skill_map.as_deref().unwrap_or("null"),
			last.lubikan.as_deref().unwrap_or("null"),
			current.lubikan.as_deref().unwrap_or("null"),
		));
	}
	push_queue_membership(&mut out, "weapons", &last.weapons, &last.weapon, &current.weapon);
	push_queue_membership(
		&mut out,
		"skill_maps",
		&last.skill_maps,
		&last.skill_map,
		&current.skill_map,
	);
	push_queue_membership(&mut out, "lubikans", &last.lubikans, &last.lubikan, &current.lubikan);
	if cycling_allowed
		&& pressed_y
		&& !pressed_category
		&& last.cycle_category == current.cycle_category
	{
		match last.cycle_category.as_str() {
			"weapons" => push_cross_category(
				&mut out,
				"weapons",
				last.skill_map != current.skill_map,
				last.lubikan != current.lubikan,
			),
			"skill_maps" => push_cross_category(
				&mut out,
				"skill_maps",
				last.weapon != current.weapon,
				last.lubikan != current.lubikan,
			),
			"lubikans" => push_cross_category(
				&mut out,
				"lubikans",
				last.weapon != current.weapon,
				last.skill_map != current.skill_map,
			),
			_ => {}
		}
	}
	out
}

fn push_queue_membership(
	out: &mut Vec<String>,
	category: &str,
	queue: &[String],
	last_item: &Option<String>,
	current_item: &Option<String>,
) {
	if last_item == current_item || queue.is_empty() {
		return;
	}
	let Some(item) = current_item else {
		return;
	};
	if !queue.contains(item) {
		out.push(format!("cycled {item} was not in the {category} queue"));
	}
}

fn push_cross_category(out: &mut Vec<String>, category: &str, other_a: bool, other_b: bool) {
	if other_a || other_b {
		out.push(format!("Y cycling {category} changed another category's active item"));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn snap(
		category: &str,
		weapon: Option<&str>,
		skill_map: Option<&str>,
		lubikan: Option<&str>,
		weapons: &[&str],
		skill_maps: &[&str],
		lubikans: &[&str],
	) -> InventoryObservation {
		InventoryObservation {
			cycle_category: category.into(),
			weapon: weapon.map(str::to_string),
			skill_map: skill_map.map(str::to_string),
			lubikan: lubikan.map(str::to_string),
			weapons: weapons.iter().map(|s| (*s).to_string()).collect(),
			skill_maps: skill_maps.iter().map(|s| (*s).to_string()).collect(),
			lubikans: lubikans.iter().map(|s| (*s).to_string()).collect(),
			invariants: Vec::new(),
		}
	}

	#[test]
	fn y_never_changes_the_category() {
		let last = snap(
			"weapons",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup"],
			&["fireball"],
			&["self"],
		);
		let current = snap(
			"lubikans",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup"],
			&["fireball"],
			&["self"],
		);
		let hits = cycling_invariants(&last, &current, true, false, true);
		assert!(hits.iter().any(|detail| detail.contains("Y changed cycle category")), "{hits:?}");
	}

	#[test]
	fn dpad_never_changes_the_active_item() {
		let last = snap(
			"weapons",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup", "reltor"],
			&["fireball"],
			&["self"],
		);
		let current = snap(
			"skill_maps",
			Some("reltor"),
			Some("fireball"),
			Some("self"),
			&["reltor", "bullpup"],
			&["fireball"],
			&["self"],
		);
		let hits = cycling_invariants(&last, &current, false, true, true);
		assert!(hits.iter().any(|detail| detail.contains("D-pad changed active item")), "{hits:?}");
	}

	#[test]
	fn cycle_stays_inside_the_category_queue() {
		let last = snap(
			"weapons",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup", "reltor"],
			&["fireball"],
			&["self"],
		);
		let current = snap(
			"weapons",
			Some("snailer"),
			Some("fireball"),
			Some("self"),
			&["snailer", "bullpup"],
			&["fireball"],
			&["self"],
		);
		let hits = cycling_invariants(&last, &current, false, false, true);
		assert!(
			hits.iter()
				.any(|detail| detail.contains("cycled snailer was not in the weapons queue")),
			"{hits:?}"
		);
	}

	#[test]
	fn y_in_weapons_does_not_change_skill_map() {
		let last = snap(
			"weapons",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup", "reltor"],
			&["fireball", "dumbwave"],
			&["self"],
		);
		let current = snap(
			"weapons",
			Some("reltor"),
			Some("dumbwave"),
			Some("self"),
			&["reltor", "bullpup"],
			&["dumbwave", "fireball"],
			&["self"],
		);
		let hits = cycling_invariants(&last, &current, true, false, true);
		assert!(
			hits.iter()
				.any(|detail| detail.contains("Y cycling weapons changed another category")),
			"{hits:?}"
		);
	}

	#[test]
	fn first_publish_is_not_an_invariant() {
		let last = InventoryObservation::default();
		let current = snap("weapons", Some("bullpup"), None, None, &["bullpup"], &[], &[]);
		assert!(cycling_invariants(&last, &current, true, true, true).is_empty());
	}

	#[test]
	fn healthy_y_cycle_is_silent() {
		let last = snap(
			"weapons",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup", "reltor"],
			&["fireball"],
			&["self"],
		);
		let current = snap(
			"weapons",
			Some("reltor"),
			Some("fireball"),
			Some("self"),
			&["reltor", "bullpup"],
			&["fireball"],
			&["self"],
		);
		assert!(cycling_invariants(&last, &current, true, false, true).is_empty());
	}

	#[test]
	fn healthy_dpad_select_is_silent() {
		let last = snap(
			"weapons",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup"],
			&["fireball"],
			&["self"],
		);
		let current = snap(
			"lubikans",
			Some("bullpup"),
			Some("fireball"),
			Some("self"),
			&["bullpup"],
			&["fireball"],
			&["self"],
		);
		assert!(cycling_invariants(&last, &current, false, true, true).is_empty());
	}
}
