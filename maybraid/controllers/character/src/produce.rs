//! [`VirtualPad`] → [`CharacterIntent`].

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureStack, InputSource, LocalActor};
use maybraid_input::{PadButton, VirtualPad, VirtualPadConfig};

use crate::intent::{CharacterIntent, InventoryCyclingCategory};
use crate::OrdinaryLocomotion;

const ANALOG_EPS: f32 = 1e-6;
const STANCE_HOLD_SECS: f32 = 0.35;

pub fn produce_character_intents(
	pad: Res<VirtualPad>,
	config: Res<VirtualPadConfig>,
	time: Res<Time>,
	capture: Option<Res<CaptureStack>>,
	local: Option<Res<LocalActor>>,
	actors: Query<&OrdinaryLocomotion>,
	mut intents: MessageWriter<CharacterIntent>,
) {
	let source = InputSource::local_pad();
	let captured = capture.as_ref().is_some_and(|stack| stack.blocks_actor(source));
	let stance_enabled = !capture.as_ref().is_some_and(|stack| stack.owns_back(source));
	let ordinary = local
		.as_deref()
		.and_then(|local| actors.get(local.entity()).ok())
		.is_none_or(|flag| flag.0);
	for intent in collect(
		&pad,
		config.trigger_press_threshold,
		time.delta_secs(),
		stance_enabled,
		ordinary,
		captured,
	) {
		intents.write(intent);
	}
}

pub fn collect(
	pad: &VirtualPad,
	trigger_threshold: f32,
	dt: f32,
	stance_enabled: bool,
	ordinary_locomotion: bool,
	actor_captured: bool,
) -> Vec<CharacterIntent> {
	let mut out = Vec::new();
	let ordinary_locomotion = ordinary_locomotion && !actor_captured;
	let stance_enabled = stance_enabled && !actor_captured;
	if ordinary_locomotion && pad.move_stick.length_squared() > ANALOG_EPS {
		out.push(CharacterIntent::Move(pad.move_stick));
	}
	if !actor_captured && pad.look_stick.length_squared() > ANALOG_EPS {
		out.push(CharacterIntent::Look(pad.look_stick));
	}
	if !actor_captured && pad.trigger_focus > ANALOG_EPS {
		out.push(CharacterIntent::Focus(pad.trigger_focus));
	}
	if !actor_captured && pad.pressed(PadButton::BumperFocus) && !pad.bumper_chord_held() {
		out.push(CharacterIntent::Ads(1.0));
	}
	if !actor_captured
		&& (pad.pressed(PadButton::TriggerFire) || pad.trigger_fire >= trigger_threshold)
	{
		out.push(CharacterIntent::UseItem(pad.trigger_fire.max(trigger_threshold)));
	}

	if ordinary_locomotion && pad.pressed(PadButton::StickClickMove) {
		out.push(CharacterIntent::StartSprint);
	}
	if ordinary_locomotion && pad.just_released(PadButton::StickClickMove) {
		out.push(CharacterIntent::StopSprint);
	}
	if !actor_captured && pad.just_pressed(PadButton::StickClickLook) {
		out.push(CharacterIntent::SwapPov);
	}
	if ordinary_locomotion && pad.just_pressed(PadButton::A) {
		out.push(CharacterIntent::Jump);
	}
	if ordinary_locomotion && stance_enabled {
		if pad.just_released(PadButton::B) && pad.hold_secs(PadButton::B) < STANCE_HOLD_SECS {
			out.push(CharacterIntent::ChangeSquat);
		}
		if pad.pressed(PadButton::B)
			&& pad.hold_secs(PadButton::B) >= STANCE_HOLD_SECS
			&& pad.hold_secs(PadButton::B) - dt < STANCE_HOLD_SECS
		{
			out.push(CharacterIntent::ChangeProne);
		}
	}
	if !actor_captured && pad.just_pressed(PadButton::X) {
		if pad.pressed(PadButton::TriggerFire) || pad.trigger_fire >= trigger_threshold {
			out.push(CharacterIntent::PowerUseItem);
		} else {
			out.push(CharacterIntent::StartInteraction);
		}
	}
	if !actor_captured && pad.just_pressed(PadButton::Y) {
		out.push(CharacterIntent::SwapActive);
	}
	// D-Pad selects the equipped queue **Y** cycles. Arrow keys mirror D-Pad for menus/walk only.
	if !actor_captured
		&& pad.just_pressed(PadButton::DpadLeft)
		&& !pad.keys.just_pressed(KeyCode::ArrowLeft)
	{
		out.push(CharacterIntent::SelectInventoryCyclingCategory(
			InventoryCyclingCategory::Lubikans,
		));
	}
	if !actor_captured
		&& pad.just_pressed(PadButton::DpadRight)
		&& !pad.keys.just_pressed(KeyCode::ArrowRight)
	{
		out.push(CharacterIntent::SelectInventoryCyclingCategory(
			InventoryCyclingCategory::Weapons,
		));
	}
	if !actor_captured
		&& pad.just_pressed(PadButton::DpadDown)
		&& !pad.keys.just_pressed(KeyCode::ArrowDown)
	{
		out.push(CharacterIntent::SelectInventoryCyclingCategory(
			InventoryCyclingCategory::SkillMaps,
		));
	}
	if !actor_captured && pad.keys.just_pressed(KeyCode::BracketLeft) {
		out.push(CharacterIntent::CycleEquippedQueue(-1));
	}
	if !actor_captured && pad.keys.just_pressed(KeyCode::BracketRight) {
		out.push(CharacterIntent::CycleEquippedQueue(1));
	}
	if pad.just_pressed(PadButton::Start) {
		out.push(CharacterIntent::InGameMenu);
	}
	// Tab also holds Select. Keep inventory on the key; Xbox View (pad Select) is the map.
	if pad.keys.just_pressed(KeyCode::Tab) {
		out.push(CharacterIntent::Inventory);
	}
	if pad.just_pressed(PadButton::Select) && !pad.keys.just_pressed(KeyCode::Tab) {
		out.push(CharacterIntent::ToggleMap);
	}
	if pad.keys.just_pressed(KeyCode::KeyM) {
		out.push(CharacterIntent::ToggleMap);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_input::VirtualPad;

	fn finish(pad: &mut VirtualPad) {
		pad.finish_digital();
	}

	fn intents(pad: &VirtualPad) -> Vec<CharacterIntent> {
		collect(pad, 0.5, 0.0, true, true, false)
	}

	#[test]
	fn move_and_look_are_analog() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::new(1.0, 0.0));
		pad.add_look(Vec2::new(0.0, -0.5));
		finish(&mut pad);
		let intents = intents(&pad);
		assert_eq!(intents[0], CharacterIntent::Move(Vec2::new(1.0, 0.0)));
		assert_eq!(intents[1], CharacterIntent::Look(Vec2::new(0.0, -0.5)));
		Ok(())
	}

	#[test]
	fn l3_hold_is_sprint_edges() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::StickClickMove);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::StartSprint]);

		pad.begin_frame();
		pad.hold_digital(PadButton::StickClickMove);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::StartSprint]);

		pad.begin_frame();
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::StopSprint]);
		Ok(())
	}

	#[test]
	fn r3_click_swaps_pov() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::StickClickLook);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::SwapPov]);
		Ok(())
	}

	#[test]
	fn face_and_menu_buttons() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.hold_digital(PadButton::B);
		pad.hold_digital(PadButton::Y);
		pad.hold_digital(PadButton::Start);
		pad.hold_digital(PadButton::Select);
		finish(&mut pad);
		assert_eq!(
			intents(&pad),
			vec![
				CharacterIntent::Jump,
				CharacterIntent::SwapActive,
				CharacterIntent::InGameMenu,
				CharacterIntent::ToggleMap,
			]
		);
		Ok(())
	}

	#[test]
	fn b_tap_is_change_squat_not_exit() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		finish(&mut pad);
		pad.tick_holds(0.1);
		assert!(!intents(&pad).contains(&CharacterIntent::ChangeSquat));
		assert!(!intents(&pad).contains(&CharacterIntent::ExitInteraction));

		pad.begin_frame();
		finish(&mut pad);
		let collected = intents(&pad);
		assert!(collected.contains(&CharacterIntent::ChangeSquat));
		assert!(!collected.contains(&CharacterIntent::ChangeProne));
		assert!(!collected.contains(&CharacterIntent::ExitInteraction));
		Ok(())
	}

	#[test]
	fn b_hold_is_change_prone_once() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		finish(&mut pad);
		pad.tick_holds(0.0);

		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		finish(&mut pad);
		pad.tick_holds(0.2);
		assert!(!collect(&pad, 0.5, 0.2, true, true, false).contains(&CharacterIntent::ChangeProne));

		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		finish(&mut pad);
		pad.tick_holds(0.2);
		let crossing = collect(&pad, 0.5, 0.2, true, true, false);
		assert!(crossing.contains(&CharacterIntent::ChangeProne));
		assert!(!crossing.contains(&CharacterIntent::ChangeSquat));

		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		finish(&mut pad);
		pad.tick_holds(0.16);
		assert!(
			!collect(&pad, 0.5, 0.16, true, true, false).contains(&CharacterIntent::ChangeProne)
		);

		pad.begin_frame();
		finish(&mut pad);
		let release = collect(&pad, 0.5, 0.16, true, true, false);
		assert!(!release.contains(&CharacterIntent::ChangeSquat));
		assert!(!release.contains(&CharacterIntent::ChangeProne));
		Ok(())
	}

	#[test]
	fn menu_or_disabled_pad_skips_stance() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		finish(&mut pad);
		pad.tick_holds(0.1);
		pad.begin_frame();
		finish(&mut pad);
		assert!(collect(&pad, 0.5, 0.1, false, true, false).is_empty());
		Ok(())
	}

	#[test]
	fn x_without_rt_starts_interaction() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::X);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::StartInteraction]);
		Ok(())
	}

	#[test]
	fn rest_analog_does_not_use_item() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.0, 0.02);
		pad.apply_trigger_digital(0.5);
		finish(&mut pad);
		assert!(!intents(&pad).iter().any(|intent| matches!(intent, CharacterIntent::UseItem(_))));
		Ok(())
	}

	#[test]
	fn squeezed_trigger_uses_item() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.0, 0.9);
		pad.apply_trigger_digital(0.5);
		finish(&mut pad);
		assert!(intents(&pad).contains(&CharacterIntent::UseItem(0.9)));
		Ok(())
	}

	#[test]
	fn rt_plus_x_is_power_use_not_interact() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.0, 0.9);
		pad.apply_trigger_digital(0.5);
		pad.hold_digital(PadButton::X);
		finish(&mut pad);
		let intents = intents(&pad);
		assert!(intents.contains(&CharacterIntent::UseItem(0.9)));
		assert!(intents.contains(&CharacterIntent::PowerUseItem));
		assert!(!intents.contains(&CharacterIntent::StartInteraction));
		Ok(())
	}

	#[test]
	fn left_trigger_is_focus() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.4, 0.0);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::Focus(0.4)]);
		Ok(())
	}

	#[test]
	fn left_bumper_is_iron_ads() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::Ads(1.0)]);
		Ok(())
	}

	#[test]
	fn both_bumpers_do_not_open_the_skill_map() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		pad.hold_digital(PadButton::BumperFire);
		finish(&mut pad);
		assert!(intents(&pad).is_empty());
		Ok(())
	}

	#[test]
	fn tanzer_skips_move_jump_and_stance_but_keeps_look() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::Y);
		pad.add_look(Vec2::X);
		pad.hold_digital(PadButton::A);
		finish(&mut pad);
		let collected = collect(&pad, 0.5, 0.0, true, false, false);
		assert!(!collected.iter().any(|intent| matches!(intent, CharacterIntent::Move(_))));
		assert!(!collected.contains(&CharacterIntent::Jump));
		assert_eq!(
			collected
				.iter()
				.find(|intent| matches!(intent, CharacterIntent::Look(_)))
				.copied(),
			Some(CharacterIntent::Look(Vec2::X))
		);
		Ok(())
	}

	#[test]
	fn dpad_selects_inventory_cycling_categories() -> anyhow::Result<()> {
		use crate::intent::InventoryCyclingCategory;

		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::DpadRight);
		finish(&mut pad);
		assert_eq!(
			intents(&pad),
			vec![CharacterIntent::SelectInventoryCyclingCategory(
				InventoryCyclingCategory::Weapons
			)]
		);

		pad.begin_frame();
		pad.hold_digital(PadButton::DpadLeft);
		finish(&mut pad);
		assert_eq!(
			intents(&pad),
			vec![CharacterIntent::SelectInventoryCyclingCategory(
				InventoryCyclingCategory::Lubikans
			)]
		);

		pad.begin_frame();
		pad.hold_digital(PadButton::DpadDown);
		finish(&mut pad);
		assert_eq!(
			intents(&pad),
			vec![CharacterIntent::SelectInventoryCyclingCategory(
				InventoryCyclingCategory::SkillMaps
			)]
		);
		Ok(())
	}

	#[test]
	fn arrow_keys_do_not_select_inventory_categories() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::ArrowLeft);
		pad.keys = keys;
		pad.begin_frame();
		pad.hold_digital(PadButton::DpadLeft);
		finish(&mut pad);
		assert!(!intents(&pad)
			.iter()
			.any(|intent| matches!(intent, CharacterIntent::SelectInventoryCyclingCategory(_))));
		Ok(())
	}

	#[test]
	fn brackets_cycle_the_equipped_queue() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::BracketRight);
		pad.keys = keys;
		pad.begin_frame();
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::CycleEquippedQueue(1)]);
		Ok(())
	}

	#[test]
	fn m_toggles_the_world_map() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::KeyM);
		pad.keys = keys;
		pad.begin_frame();
		finish(&mut pad);
		assert!(intents(&pad).contains(&CharacterIntent::ToggleMap));
		Ok(())
	}

	#[test]
	fn select_toggles_the_world_map() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::Select);
		finish(&mut pad);
		assert_eq!(intents(&pad), vec![CharacterIntent::ToggleMap]);
		Ok(())
	}

	#[test]
	fn dpad_up_does_not_toggle_the_world_map() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::DpadUp);
		finish(&mut pad);
		assert!(!intents(&pad).contains(&CharacterIntent::ToggleMap));
		Ok(())
	}

	#[test]
	fn tab_still_opens_inventory() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::Tab);
		pad.keys = keys;
		pad.begin_frame();
		pad.hold_digital(PadButton::Select);
		finish(&mut pad);
		assert!(intents(&pad).contains(&CharacterIntent::Inventory));
		assert!(!intents(&pad).contains(&CharacterIntent::ToggleMap));
		Ok(())
	}

	#[test]
	fn capture_skips_realization_but_keeps_start_and_map() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::Y);
		pad.add_look(Vec2::X);
		pad.hold_digital(PadButton::A);
		pad.hold_digital(PadButton::Start);
		pad.hold_digital(PadButton::Select);
		pad.max_triggers(0.0, 0.9);
		pad.apply_trigger_digital(0.5);
		finish(&mut pad);
		let collected = collect(&pad, 0.5, 0.0, true, true, true);
		assert!(!collected.iter().any(|intent| matches!(intent, CharacterIntent::Move(_))));
		assert!(!collected.iter().any(|intent| matches!(intent, CharacterIntent::Look(_))));
		assert!(!collected.iter().any(|intent| matches!(intent, CharacterIntent::UseItem(_))));
		assert!(!collected.contains(&CharacterIntent::Jump));
		assert!(collected.contains(&CharacterIntent::InGameMenu));
		assert!(collected.contains(&CharacterIntent::ToggleMap));
		Ok(())
	}
}
