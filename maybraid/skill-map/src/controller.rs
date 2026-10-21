//! Stick-flick detector. Either stick while **RB** is held; a hold is not a flick.
//! Emits actor/session-addressed [`SkillMapFlick`].

use bevy::prelude::*;
use maybraid_character_controller::{ActorControlSample, OrdinaryLocomotion, RoutedActorControl};
use maybraid_controllers_core::{
	ActorTarget, GestureEffect, InputRouter, InputSource, RoutingEvent, RoutingKind,
};

use crate::user::SkillMapUser;
use crate::SkillMapEnabled;

/// Rest nest. Below this the stick has come home.
pub const FLICK_REST: f32 = 0.28;
/// Peak must clear this or it was a nudge.
pub const FLICK_MIN_PEAK: f32 = 0.28;
/// Faster than a hold, slower than a one-frame tap.
pub const FLICK_MIN_SECS: f32 = 0.02;
pub const FLICK_MAX_SECS: f32 = 0.32;

/// One completed flick. Direction is the stick throw; length is how hard (0..1+).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SkillMapFlick {
	actor: ActorTarget,
	session: Option<Entity>,
	throw: Vec2,
}

impl SkillMapFlick {
	pub fn new(actor: impl Into<ActorTarget>, session: Option<Entity>, throw: Vec2) -> Self {
		Self { actor: actor.into(), session, throw }
	}

	pub const fn actor(self) -> ActorTarget {
		self.actor
	}

	pub const fn session(self) -> Option<Entity> {
		self.session
	}

	pub const fn throw(self) -> Vec2 {
		self.throw
	}

	pub fn addresses(self, user: Entity, session: Entity) -> bool {
		if let Some(target) = self.session {
			return target == session;
		}
		self.actor.matches(user)
	}
}

#[derive(Clone, Debug, Default)]
struct FlickLane {
	active: bool,
	t0: f32,
	peak: Vec2,
}

impl FlickLane {
	fn reset(&mut self) {
		self.active = false;
		self.peak = Vec2::ZERO;
	}

	fn sample(&mut self, t: f32, stick: Vec2) -> Option<Vec2> {
		let mag = stick.length();
		if !self.active {
			if mag >= FLICK_REST {
				self.active = true;
				self.t0 = t;
				self.peak = stick;
			}
			return None;
		}
		if mag > self.peak.length() {
			self.peak = stick;
		}
		let elapsed = t - self.t0;
		if elapsed > FLICK_MAX_SECS {
			if mag < FLICK_REST {
				self.reset();
			}
			return None;
		}
		if mag >= FLICK_REST {
			return None;
		}
		self.active = false;
		let peak = self.peak;
		self.peak = Vec2::ZERO;
		if elapsed >= FLICK_MIN_SECS && peak.length() >= FLICK_MIN_PEAK {
			Some(peak)
		} else {
			None
		}
	}
}

/// How the map consumes stick input.
///
/// Standard keeps RB + either-stick flicks. Tanz disables that producer so
/// committed Tanz gestures can publish [`SkillMapFlick`] instead.
///
/// Temporary until phase 4: a global policy adapter. Per-actor
/// [`OrdinaryLocomotion`] already gates Standard flick.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillMapSteerPolicy {
	#[default]
	Standard,
	Tanz,
}

/// True while the skill-map steer modifier is down (Xbox **RB** / keyboard **V**)
/// and the Tanzer bumper chord is not held.
pub fn skill_map_steer_held(sample: ActorControlSample) -> bool {
	sample.skill_map_steer_held()
}

/// Watches move and look sticks. One flick per sample, stronger stick wins if both land.
///
/// History is scoped to one `(source, actor)` owner and the session the
/// gesture started against. Changing session cannot complete against the new
/// session. `RoutingEvent` cancel / freeze apply here.
#[derive(Resource, Clone, Debug, Default)]
pub struct SkillMapController {
	owner: Option<(InputSource, ActorTarget)>,
	session: Option<Entity>,
	frozen: bool,
	move_lane: FlickLane,
	look_lane: FlickLane,
}

impl SkillMapController {
	pub fn reset(&mut self) {
		self.owner = None;
		self.session = None;
		self.frozen = false;
		self.move_lane.reset();
		self.look_lane.reset();
	}

	pub const fn owner(&self) -> Option<(InputSource, ActorTarget)> {
		self.owner
	}

	pub const fn session(&self) -> Option<Entity> {
		self.session
	}

	pub const fn is_frozen(&self) -> bool {
		self.frozen
	}

	pub fn has_partial(&self) -> bool {
		self.move_lane.active || self.look_lane.active
	}

	/// Apply a router hold / gesture contract for this source.
	pub fn apply_routing(&mut self, event: RoutingEvent) {
		if self.owner.is_some_and(|(source, _)| source != event.source()) {
			return;
		}
		if self.owner.is_none() && !self.has_partial() && !self.frozen {
			return;
		}
		match event.gesture() {
			GestureEffect::Cancel => self.reset(),
			GestureEffect::Freeze => self.frozen = true,
			GestureEffect::None => {
				if matches!(event.kind(), RoutingKind::Assigned | RoutingKind::Resumed) {
					self.frozen = false;
				}
			}
		}
	}

	/// Bind this frame's owner and session. Returns whether sampling may complete.
	///
	/// A session change drops in-flight history so the return-to-rest cannot
	/// emit a flick addressed at the new session.
	pub fn bind_owner(
		&mut self,
		source: InputSource,
		actor: ActorTarget,
		session: Option<Entity>,
	) -> bool {
		if self.frozen {
			return false;
		}
		if self.owner != Some((source, actor)) {
			self.move_lane.reset();
			self.look_lane.reset();
			self.owner = Some((source, actor));
			self.session = session;
			return true;
		}
		if self.session != session {
			self.move_lane.reset();
			self.look_lane.reset();
			self.session = session;
			return false;
		}
		true
	}

	pub fn sample(&mut self, t: f32, move_stick: Vec2, look_stick: Vec2) -> Option<Vec2> {
		let moved = self.move_lane.sample(t, move_stick);
		let looked = self.look_lane.sample(t, look_stick);
		match (moved, looked) {
			(Some(a), Some(b)) if b.length_squared() > a.length_squared() => Some(b),
			(Some(a), _) => Some(a),
			(None, Some(b)) => Some(b),
			(None, None) => None,
		}
	}
}

pub fn detect_skill_map_flicks(
	time: Res<Time>,
	router: Option<Res<InputRouter>>,
	routed: Option<Res<RoutedActorControl>>,
	enabled: Res<SkillMapEnabled>,
	users: Query<(Entity, &SkillMapUser, Option<&OrdinaryLocomotion>)>,
	mut controller: ResMut<SkillMapController>,
	mut flicks: MessageWriter<SkillMapFlick>,
) {
	if let Some(router) = router.as_deref() {
		for event in router.events() {
			controller.apply_routing(*event);
		}
	}
	if !enabled.0 {
		controller.reset();
		return;
	}
	if controller.is_frozen() {
		return;
	}
	let Some(routed) = routed.as_deref() else {
		return;
	};
	let Some(actor) = routed.actor() else {
		return;
	};
	let Some(source) = routed.source() else {
		return;
	};
	let Some(sample) = routed.payload() else {
		return;
	};
	let Ok((_, user, ordinary)) = users.get(actor.entity()) else {
		controller.reset();
		return;
	};
	if ordinary.is_some_and(|flag| !flag.0) || !skill_map_steer_held(sample) {
		controller.reset();
		return;
	}
	if !controller.bind_owner(source, actor, Some(user.maps)) {
		return;
	}
	if let Some(flick) =
		controller.sample(time.elapsed_secs(), sample.move_stick(), sample.look_throw())
	{
		flicks.write(SkillMapFlick::new(actor, controller.session(), flick));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn throw(
		controller: &mut SkillMapController,
		t0: f32,
		stick: Vec2,
		on_move: bool,
	) -> Option<Vec2> {
		let mut last = None;
		for (i, mag) in [0.0, 0.75, 1.0, 0.4, 0.0].into_iter().enumerate() {
			let t = t0 + i as f32 * 0.04;
			let v = stick * mag;
			last = if on_move {
				controller.sample(t, v, Vec2::ZERO)
			} else {
				controller.sample(t, Vec2::ZERO, v)
			};
		}
		last
	}

	#[test]
	fn a_fast_throw_on_either_stick_is_a_flick() {
		let mut controller = SkillMapController::default();
		let moved = throw(&mut controller, 0.0, Vec2::X, true).expect("move flick");
		assert!(moved.x > 0.9);
		let looked = throw(&mut controller, 1.0, Vec2::Y, false).expect("look flick");
		assert!(looked.y > 0.9);
	}

	#[test]
	fn a_held_stick_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.3, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.35, Vec2::ZERO, Vec2::ZERO).is_none());
	}

	#[test]
	fn rb_is_the_steer_hold() {
		assert!(!skill_map_steer_held(ActorControlSample::default()));
		assert!(skill_map_steer_held(ActorControlSample::default().with_bumpers(false, true)));
	}

	#[test]
	fn bumper_chord_is_not_the_steer_hold() {
		assert!(!skill_map_steer_held(ActorControlSample::default().with_bumpers(true, true)));
	}

	#[test]
	fn a_nudge_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X * 0.15, Vec2::ZERO).is_none());
		assert!(controller.sample(0.08, Vec2::X * 0.2, Vec2::ZERO).is_none());
		assert!(controller.sample(0.12, Vec2::ZERO, Vec2::ZERO).is_none());
	}

	#[test]
	fn changing_session_does_not_complete_against_the_new_session() {
		let mut controller = SkillMapController::default();
		let actor = ActorTarget::new(Entity::from_bits(1));
		let source = InputSource::local_pad();
		let session_a = Entity::from_bits(10);
		let session_b = Entity::from_bits(11);
		assert!(controller.bind_owner(source, actor, Some(session_a)));
		assert!(controller.sample(0.00, Vec2::X * 0.0, Vec2::ZERO).is_none());
		assert!(controller.sample(0.04, Vec2::X * 0.75, Vec2::ZERO).is_none());
		assert!(controller.sample(0.08, Vec2::X * 1.0, Vec2::ZERO).is_none());
		assert!(controller.has_partial());
		assert!(!controller.bind_owner(source, actor, Some(session_b)));
		assert!(!controller.has_partial());
		assert_eq!(controller.session(), Some(session_b));
		assert!(
			controller.sample(0.12, Vec2::ZERO, Vec2::ZERO).is_none(),
			"return-to-rest after a session change is not a completion"
		);
	}

	#[test]
	fn reassignment_cancels_a_partial_flick() {
		let mut controller = SkillMapController::default();
		let source = InputSource::local_pad();
		let actor_a = ActorTarget::new(Entity::from_bits(1));
		assert!(controller.bind_owner(source, actor_a, Some(Entity::from_bits(9))));
		assert!(controller.sample(0.0, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.has_partial());
		let mut router = InputRouter::default();
		router.assign_actor(source, actor_a);
		router.take_events();
		router.assign_actor(source, ActorTarget::new(Entity::from_bits(2)));
		let event = router.events().last().copied().expect("reassigned");
		assert_eq!(event.kind(), RoutingKind::Reassigned);
		assert_eq!(event.gesture(), GestureEffect::Cancel);
		controller.apply_routing(event);
		assert!(!controller.has_partial());
		assert!(controller.owner().is_none());
	}
}
