//! Footsteps on grounded walk / run, cadence-matched to the locomotion clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use characters::AnimId;
use maybraid_audio::{
	Audio, AudioClip, AudioSystems, Mixer, MovementClip, MovementSounds, MovementState,
};

use crate::body::{CharacterController, Grounded, JumpPhase, Jumping, Sprinting, LEAP_SPEED};
use crate::locomotion::WALK_SPEED;
use crate::stance::{CharacterStance, StanceKind};
use crate::{Npc, Player};

pub(crate) fn ensure_movement_states(
	mut commands: Commands,
	bodies: Query<
		Entity,
		(With<CharacterController>, Without<MovementState>, Or<(With<Player>, With<Npc>)>),
	>,
) {
	for entity in &bodies {
		commands.entity(entity).insert(MovementState::seeded(entity.to_bits()));
	}
}

pub(crate) fn play_footsteps(
	mut commands: Commands,
	time: Res<Time>,
	mut bodies: Query<
		(
			Entity,
			&GlobalTransform,
			&LinearVelocity,
			&mut MovementState,
			Option<&Grounded>,
			Option<&Jumping>,
			Option<&Sprinting>,
			Option<&CharacterStance>,
		),
		With<CharacterController>,
	>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<MovementSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	let dt = time.delta_secs();
	for (_entity, transform, velocity, mut gait, grounded, jumping, sprinting, stance) in
		&mut bodies
	{
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		let rate = footstep_rate(grounded.is_some(), jumping, stance, speed);
		let foley = jump_foley(jumping, &mut gait);
		let steps = gait.take_steps(rate, dt) + foley.steps;
		let sprint_breath = if jumping.is_some() {
			None
		} else {
			gait.take_sprint_breath(
				sprint_breathing(sprinting.is_some(), grounded.is_some(), jumping, speed),
				dt,
			)
		};
		let point = transform.translation();
		for _ in 0..steps {
			sounds.play_footstep(
				&mut commands,
				&mut gait,
				&clips,
				audio,
				&mut mixer,
				listener,
				point,
			);
		}
		if foley.inhale {
			sounds.play_breath(
				&mut commands,
				MovementClip::Inhale,
				&clips,
				audio,
				&mut mixer,
				listener,
				point,
			);
		}
		if foley.exhale {
			sounds.play_breath(
				&mut commands,
				MovementClip::Exhale,
				&clips,
				audio,
				&mut mixer,
				listener,
				point,
			);
		}
		if let Some(breath) = sprint_breath {
			sounds.play_breath(&mut commands, breath, &clips, audio, &mut mixer, listener, point);
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct JumpFoley {
	pub steps: u32,
	pub inhale: bool,
	pub exhale: bool,
}

/// One plant plus inhale on takeoff, one plant plus exhale on land.
pub(crate) fn jump_foley(jumping: Option<&Jumping>, state: &mut MovementState) -> JumpFoley {
	let Some(jump) = jumping else {
		state.takeoff_planted = false;
		state.land_planted = false;
		return JumpFoley::default();
	};
	let mut foley = JumpFoley::default();
	if !state.takeoff_planted {
		state.takeoff_planted = true;
		foley.steps += 1;
		foley.inhale = true;
	}
	if jump.phase == JumpPhase::Land && !state.land_planted {
		state.land_planted = true;
		foley.steps += 1;
		foley.exhale = true;
	}
	foley
}

/// Sprint breath only while the hold is live, grounded, and actually moving.
pub(crate) fn sprint_breathing(
	sprinting: bool,
	grounded: bool,
	jumping: Option<&Jumping>,
	speed: f32,
) -> bool {
	sprinting && grounded && jumping.is_none() && speed > WALK_SPEED
}

/// Two plants per walk/run cycle. Air, stance, and idle stay quiet.
pub(crate) fn footstep_rate(
	grounded: bool,
	jumping: Option<&Jumping>,
	stance: Option<&CharacterStance>,
	speed: f32,
) -> f32 {
	if !grounded || jumping.is_some() {
		return 0.0;
	}
	let kind = stance.map(|stance| stance.kind).unwrap_or(StanceKind::Stand);
	if !matches!(kind, StanceKind::Stand) {
		return 0.0;
	}
	if speed > LEAP_SPEED {
		AnimId::Run.default_speed() * 2.0
	} else if speed > WALK_SPEED {
		AnimId::Walk.default_speed() * 2.0
	} else {
		0.0
	}
}

pub(crate) fn configure_movement(app: &mut App) {
	app.add_systems(
		PostUpdate,
		(ensure_movement_states, play_footsteps)
			.chain()
			.after(TransformSystems::Propagate)
			.before(AudioSystems::Sync),
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stand_walk_uses_two_plants_per_cycle() {
		assert!(
			(footstep_rate(true, None, None, 3.0) - AnimId::Walk.default_speed() * 2.0).abs()
				< 1e-5
		);
		assert!(
			(footstep_rate(true, None, None, 9.0) - AnimId::Run.default_speed() * 2.0).abs() < 1e-5
		);
	}

	#[test]
	fn jump_and_leap_plant_on_takeoff_and_land() {
		let mut state = MovementState::seeded(4);
		let mut hop = Jumping::start(0.0);
		assert_eq!(
			jump_foley(Some(&hop), &mut state),
			JumpFoley { steps: 1, inhale: true, exhale: false }
		);
		assert_eq!(jump_foley(Some(&hop), &mut state), JumpFoley::default());
		hop.phase = JumpPhase::Land;
		assert_eq!(
			jump_foley(Some(&hop), &mut state),
			JumpFoley { steps: 1, inhale: false, exhale: true }
		);
		assert_eq!(jump_foley(None, &mut state), JumpFoley::default());
		let mut leap = Jumping::start(9.0);
		assert!(leap.leaping);
		assert_eq!(
			jump_foley(Some(&leap), &mut state),
			JumpFoley { steps: 1, inhale: true, exhale: false }
		);
		leap.phase = JumpPhase::Air;
		assert_eq!(jump_foley(Some(&leap), &mut state), JumpFoley::default());
		leap.phase = JumpPhase::Land;
		assert_eq!(
			jump_foley(Some(&leap), &mut state),
			JumpFoley { steps: 1, inhale: false, exhale: true }
		);
		assert_eq!(jump_foley(Some(&leap), &mut state), JumpFoley::default());
	}

	#[test]
	fn sprint_breath_needs_a_live_grounded_run() {
		assert!(sprint_breathing(true, true, None, 3.0));
		assert!(!sprint_breathing(false, true, None, 9.0));
		assert!(!sprint_breathing(true, false, None, 9.0));
		assert!(!sprint_breathing(true, true, None, 0.4));
		let jump = Jumping::start(0.0);
		assert!(!sprint_breathing(true, true, Some(&jump), 9.0));
	}

	#[test]
	fn idle_air_and_stance_stay_quiet() {
		assert_eq!(footstep_rate(true, None, None, 0.4), 0.0);
		assert_eq!(footstep_rate(false, None, None, 6.0), 0.0);
		let jump = Jumping::start(0.0);
		assert_eq!(footstep_rate(true, Some(&jump), None, 6.0), 0.0);
		let squat = CharacterStance { kind: StanceKind::Squat, blend: 1.0 };
		assert_eq!(footstep_rate(true, None, Some(&squat), 6.0), 0.0);
	}

	#[test]
	fn player_controller_gets_a_gait() {
		let mut app = App::new();
		app.add_systems(Update, ensure_movement_states);
		let player = app.world_mut().spawn((Player, CharacterController)).id();
		app.update();
		assert!(app.world().get::<MovementState>(player).is_some());
	}
}
