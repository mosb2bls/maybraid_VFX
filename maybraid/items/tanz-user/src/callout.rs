//! Display-only Tanz trigger callout. Shown when a Tanz begins.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::stack::TanzId;
use crate::TanzExecution;

/// Fade in, hold, and fade out across this window.
pub const TANZ_CALLOUT_SECS: f32 = 1.25;
const FADE_IN_SECS: f32 = 0.18;
const FADE_OUT_SECS: f32 = 0.32;

/// Live HUD callout for the most recently triggered Tanz.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct TanzCallout {
	pub current: Option<TanzCalloutView>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TanzCalloutView {
	pub id: TanzId,
	pub name: &'static str,
	pub age: f32,
}

impl TanzCallout {
	pub fn show(&mut self, id: TanzId) {
		self.current = Some(TanzCalloutView { id, name: id.display_name(), age: 0.0 });
	}

	pub fn tick(&mut self, dt: f32) {
		let Some(view) = self.current.as_mut() else {
			return;
		};
		view.age += dt;
		if view.age >= TANZ_CALLOUT_SECS {
			self.current = None;
		}
	}

	pub fn alpha(&self) -> f32 {
		let Some(view) = &self.current else {
			return 0.0;
		};
		fade_alpha(view.age)
	}
}

pub fn fade_alpha(age: f32) -> f32 {
	if age <= 0.0 {
		return 0.0;
	}
	if age < FADE_IN_SECS {
		return (age / FADE_IN_SECS).clamp(0.0, 1.0);
	}
	let fade_out_at = TANZ_CALLOUT_SECS - FADE_OUT_SECS;
	if age > fade_out_at {
		return ((TANZ_CALLOUT_SECS - age) / FADE_OUT_SECS).clamp(0.0, 1.0);
	}
	1.0
}

/// Replace the callout when a Tanz begins or is accepted as a chain buffer.
pub fn announce_tanz_callouts(
	users: Query<(Entity, &TanzExecution)>,
	mut callout: ResMut<TanzCallout>,
	mut last: Local<HashMap<Entity, (Option<TanzId>, Option<TanzId>)>>,
) {
	for (entity, execution) in &users {
		let active = execution.active.map(|active| active.id);
		let buffered = execution.buffered.map(|buffered| buffered.id);
		let previous = last.get(&entity).copied().unwrap_or((None, None));
		if buffered != previous.1 {
			if let Some(id) = buffered {
				callout.show(id);
			}
		} else if active != previous.0 {
			if let Some(id) = active {
				callout.show(id);
			}
		}
		last.insert(entity, (active, buffered));
	}
}

pub fn tick_tanz_callout(time: Res<Time>, mut callout: ResMut<TanzCallout>) {
	callout.tick(time.delta_secs());
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::input::TanzButton;
	use crate::stack::TanzStack;
	use crate::{match_tanz_inputs, ControlMode, TanzDefinitionExt, TanzInput, TanzUser};

	fn matching_input(user: Entity, id: TanzId) -> TanzInput {
		let definition = id.definition();
		TanzInput {
			user,
			direction: definition.input.direction,
			magnitude: 0.9,
			coherence: 1.0,
			button: definition.input.button,
		}
	}

	fn app_with_callout() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.init_resource::<TanzCallout>()
			.add_systems(
				Update,
				(match_tanz_inputs, announce_tanz_callouts, tick_tanz_callout).chain(),
			);
		app
	}

	fn spawn_tanzer(app: &mut App) -> Entity {
		app.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
			))
			.id()
	}

	#[test]
	fn triggering_a_tanz_shows_callout_name_and_icon() -> anyhow::Result<()> {
		let mut app = app_with_callout();
		let user = spawn_tanzer(&mut app);
		app.world_mut().write_message(matching_input(user, TanzId::ForwardDash));
		app.update();
		let callout = app.world().resource::<TanzCallout>();
		let view = callout.current.as_ref().ok_or_else(|| anyhow::anyhow!("missing callout"))?;
		assert_eq!(view.id, TanzId::ForwardDash);
		assert_eq!(view.name, "Forward Dash");
		Ok(())
	}

	#[test]
	fn new_tanz_replaces_the_callout() -> anyhow::Result<()> {
		let mut app = app_with_callout();
		let user = spawn_tanzer(&mut app);
		app.world_mut().write_message(matching_input(user, TanzId::LeftStrafe));
		app.update();
		{
			let mut execution = app
				.world_mut()
				.get_mut::<TanzExecution>(user)
				.ok_or_else(|| anyhow::anyhow!("execution"))?;
			execution.tick(0.10);
		}
		app.world_mut().write_message(matching_input(user, TanzId::RightStrafe));
		app.update();
		let callout = app.world().resource::<TanzCallout>();
		let view = callout.current.as_ref().ok_or_else(|| anyhow::anyhow!("missing callout"))?;
		assert_eq!(view.id, TanzId::RightStrafe);
		assert_eq!(view.name, "Right Strafe");
		assert!(
			view.age < TANZ_CALLOUT_SECS * 0.5,
			"replacement should restart the fade {}",
			view.age
		);
		Ok(())
	}

	#[test]
	fn callout_hides_after_the_timeout() -> anyhow::Result<()> {
		let mut callout = TanzCallout::default();
		callout.show(TanzId::HealingSalute);
		assert_eq!(callout.current.as_ref().map(|view| view.name), Some("Healing Salute"));
		assert_eq!(callout.current.as_ref().map(|view| view.id), Some(TanzId::HealingSalute));
		callout.tick(0.80);
		assert!(callout.current.is_some());
		callout.tick(0.60);
		assert!(callout.current.is_none());
		Ok(())
	}

	#[test]
	fn unmatched_input_does_not_show_a_callout() -> anyhow::Result<()> {
		let mut app = app_with_callout();
		let user = spawn_tanzer(&mut app);
		app.world_mut().write_message(TanzInput {
			user,
			direction: Vec2::ZERO,
			magnitude: 0.0,
			coherence: 0.0,
			button: TanzButton::APress,
		});
		app.update();
		assert!(app.world().resource::<TanzCallout>().current.is_none());
		assert!(app
			.world()
			.get::<TanzExecution>(user)
			.is_some_and(|execution| execution.active.is_none()));
		Ok(())
	}

	#[test]
	fn fade_alpha_rises_then_falls() {
		assert_eq!(fade_alpha(0.0), 0.0);
		assert!((fade_alpha(FADE_IN_SECS * 0.5) - 0.5).abs() < 1e-4);
		assert_eq!(fade_alpha(0.5), 1.0);
		assert!(fade_alpha(TANZ_CALLOUT_SECS - 0.01) < 0.1);
		assert_eq!(fade_alpha(TANZ_CALLOUT_SECS), 0.0);
	}
}
