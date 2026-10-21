//! Player squat / prone. Hulls, speed, and clips derive from this plus the rest capsule.

use bevy::prelude::*;
use character_animations::animations::Squat;
use character_rigs::humanoid::LegSegmentLengths;
use characters::{CharacterRoot, LocomotionCapsule, TerrainPitch};
use damage::HeadshotBand;

use crate::body::{apply_locomotion_capsule, CharacterController, Jumping};

/// Rest-pose hull used to derive squat / prone capsules.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RestLocomotionCapsule(pub LocomotionCapsule);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StanceKind {
	#[default]
	Stand,
	Squat,
	Prone,
}

impl StanceKind {
	/// Target squat depth once a stance change has finished blending.
	pub fn squat_target(self) -> f32 {
		match self {
			Self::Stand => 0.0,
			Self::Squat | Self::Prone => 1.0,
		}
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CharacterStance {
	pub kind: StanceKind,
	/// 0 = previous pose, 1 = settled at `kind`.
	pub blend: f32,
}

impl CharacterStance {
	pub fn settled(kind: StanceKind) -> Self {
		Self { kind, blend: kind.squat_target() }
	}

	pub fn speed_scale(self) -> f32 {
		match self.kind {
			StanceKind::Stand => 1.0,
			StanceKind::Squat => 0.5,
			StanceKind::Prone => 0.25,
		}
	}

	pub fn change_squat(&mut self) {
		self.kind = match self.kind {
			StanceKind::Stand => StanceKind::Squat,
			StanceKind::Squat => StanceKind::Stand,
			StanceKind::Prone => StanceKind::Squat,
		};
		self.blend = match self.kind {
			StanceKind::Squat => 0.0,
			StanceKind::Stand => 1.0,
			StanceKind::Prone => 1.0,
		};
	}

	pub fn change_prone(&mut self) {
		self.kind = StanceKind::Prone;
		self.blend = 1.0;
	}

	pub fn stand(&mut self) {
		self.kind = StanceKind::Stand;
		self.blend = 0.0;
	}

	/// True once the squat depth matches the target stance.
	pub fn squat_settled(self) -> bool {
		match self.kind {
			StanceKind::Stand => self.blend <= 0.0,
			StanceKind::Squat => self.blend >= 1.0,
			StanceKind::Prone => true,
		}
	}

	/// Target squat depth for the current stance kind.
	pub fn squat_target(self) -> f32 {
		self.kind.squat_target()
	}

	pub fn is_prone(self) -> bool {
		self.kind == StanceKind::Prone
	}
}

pub fn squat_drop() -> f32 {
	Squat::held().peak_vertical_drop(LegSegmentLengths::default())
}

impl RestLocomotionCapsule {
	pub fn hull_for(self, kind: StanceKind) -> LocomotionCapsule {
		match kind {
			StanceKind::Stand => self.0,
			StanceKind::Squat => self.0.squat(squat_drop()),
			StanceKind::Prone => self.0.prone_motor(),
		}
	}
}

/// Ease squat depth toward the stance target so enter/exit uses [`SquatDescent`].
pub(crate) fn advance_stance_blend(
	time: Res<Time>,
	mut stances: Query<&mut CharacterStance, With<CharacterController>>,
) {
	let dt = time.delta_secs();
	for mut stance in &mut stances {
		let target = stance.squat_target();
		if stance.blend < target {
			stance.blend = (stance.blend
				+ dt * character_animations::animations::DEFAULT_DESCENT_SPEED)
				.min(target);
		} else if stance.blend > target {
			stance.blend = (stance.blend
				- dt * character_animations::animations::DEFAULT_DESCENT_SPEED)
				.max(target);
		}
	}
}

pub(crate) fn stand_when_airborne(mut stances: Query<(&Jumping, &mut CharacterStance)>) {
	for (jump, mut stance) in &mut stances {
		if jump.airborne() && stance.kind != StanceKind::Stand {
			stance.stand();
		}
	}
}

pub(crate) fn apply_stance_hulls(
	mut commands: Commands,
	mut bodies: Query<
		(
			Entity,
			&CharacterStance,
			&RestLocomotionCapsule,
			&LocomotionCapsule,
			&mut Transform,
			Option<&mut HeadshotBand>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, stance, rest, current, mut transform, band) in &mut bodies {
		let next = rest.hull_for(stance.kind);
		// Re-stamp / respawn can write a standing hull without changing stance.
		// Reconcile every frame so squat/prone take effect again after that.
		if *current == next {
			continue;
		}
		transform.translation += current.origin_delta(next);
		apply_locomotion_capsule(&mut commands, entity, next);
		match stance.kind {
			StanceKind::Prone => {
				if !rest.0.pronograde {
					commands.entity(entity).insert(rest.0.prone_hit_capsule());
					commands.entity(entity).remove::<characters::HeadCapsule>();
				}
				commands.entity(entity).remove::<HeadshotBand>();
			}
			StanceKind::Stand | StanceKind::Squat => {
				let min_local_y = next.headshot_min_local_y();
				if let Some(mut band) = band {
					band.min_local_y = min_local_y;
				} else if (current.half_height() - current.radius).abs() < 1e-4 {
					commands.entity(entity).insert(HeadshotBand { min_local_y, multiplier: 1.25 });
				}
			}
		}
	}
}

/// Prone uses a long wheelbase and full pitch weight so the visual follows slope.
pub(crate) fn sync_stance_pitch(
	stances: Query<(Entity, &CharacterStance, &RestLocomotionCapsule), With<CharacterController>>,
	mut visuals: Query<(&mut TerrainPitch, &ChildOf), With<CharacterRoot>>,
) {
	for (body, stance, rest) in &stances {
		for (mut pitch, child) in &mut visuals {
			if child.parent() != body {
				continue;
			}
			match stance.kind {
				StanceKind::Prone => {
					pitch.pitch_weight = 1.0;
					pitch.half_span = rest.0.half_height().max(0.7);
					pitch.support_locked = false;
				}
				StanceKind::Stand | StanceKind::Squat => {
					pitch.pitch_weight = 0.4;
					pitch.half_span = 0.22;
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::apply_character_controller;
	use crate::hit::{maintain_hit_volumes, HitVolume};
	use anyhow::{anyhow, Result};
	use avian3d::prelude::{Collider, Sensor};
	use bevy::ecs::system::RunSystemOnce;
	use characters::HitCapsule;

	#[test]
	fn speed_scale_slows_squat_and_prone() {
		assert!(CharacterStance::settled(StanceKind::Squat).speed_scale() < 1.0);
		assert!(CharacterStance::settled(StanceKind::Prone).speed_scale() < 1.0);
		assert!((CharacterStance::settled(StanceKind::Stand).speed_scale() - 1.0).abs() < 1e-6);
	}

	#[test]
	fn squat_hull_is_shorter() -> Result<()> {
		let rest = LocomotionCapsule::HUMANOID;
		let next = rest.squat(squat_drop());
		if (next.radius - rest.radius).abs() > 1e-5 {
			return Err(anyhow!("squat keeps radius"));
		}
		if next.half_height() >= rest.half_height() {
			return Err(anyhow!("squat half_height should shrink"));
		}
		if rest.origin_delta(next).y >= 0.0 {
			return Err(anyhow!("origin should drop when squatting"));
		}
		Ok(())
	}

	#[test]
	fn prone_hull_is_low_and_long() -> Result<()> {
		let rest = LocomotionCapsule::HUMANOID;
		let motor = rest.prone_motor();
		if (motor.half_height() - rest.radius).abs() > 1e-5 {
			return Err(anyhow!("prone motor half_height should equal radius"));
		}
		let hit = rest.prone_hit_capsule();
		if hit.local_transform().rotation.x.abs() < 0.5 {
			return Err(anyhow!("prone hit hull should pitch about X"));
		}
		if hit.length + 2.0 * hit.radius < rest.half_height() {
			return Err(anyhow!("prone query hull should be on the order of standing height"));
		}
		let rest_head = rest.headshot_min_local_y();
		if rest_head <= motor.half_height() {
			return Err(anyhow!("standing headshot Y should sit above the prone motor AABB"));
		}
		Ok(())
	}

	#[test]
	fn feet_stay_planted_squat_then_stand() -> Result<()> {
		let rest = LocomotionCapsule::HUMANOID;
		let squat = rest.squat(squat_drop());
		let mut origin = Vec3::new(0.0, rest.half_height(), 0.0);
		let feet = origin.y - rest.half_height();
		origin += rest.origin_delta(squat);
		if (origin.y - squat.half_height() - feet).abs() > 1e-4 {
			return Err(anyhow!("squat should keep feet planted"));
		}
		origin += squat.origin_delta(rest);
		if (origin.y - rest.half_height() - feet).abs() > 1e-4 {
			return Err(anyhow!("stand should restore the same feet Y"));
		}
		Ok(())
	}

	#[test]
	fn apply_squat_hull_has_no_hit_capsule() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, maintain_hit_volumes);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		let squat = LocomotionCapsule::HUMANOID.squat(squat_drop());
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
				apply_locomotion_capsule(&mut commands, body, squat);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		if app.world().get::<HitCapsule>(body).is_some() {
			return Err(anyhow!("squat humanoid must not grow a HitCapsule"));
		}
		let hull = app
			.world()
			.get::<LocomotionCapsule>(body)
			.ok_or_else(|| anyhow!("missing hull"))?;
		if (hull.length - squat.length).abs() > 1e-5 {
			return Err(anyhow!("live hull length should match squat"));
		}
		if app.world().get::<Collider>(body).is_none() {
			return Err(anyhow!("squat should keep a motor Collider"));
		}
		Ok(())
	}

	#[test]
	fn apply_prone_stamps_a_body_hit_volume() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, (apply_stance_hulls, maintain_hit_volumes).chain());
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		app.world_mut()
			.entity_mut(body)
			.insert(CharacterStance::settled(StanceKind::Prone));
		app.update();
		if app.world().get::<HitCapsule>(body).is_none() {
			return Err(anyhow!("prone should insert a query HitCapsule"));
		}
		let volumes = {
			let mut query = app.world_mut().query::<(&HitVolume, &ChildOf, &Sensor)>();
			query.iter(app.world()).filter(|(_, child, _)| child.parent() == body).count()
		};
		if volumes != 1 {
			return Err(anyhow!("prone should stamp one body HitVolume, got {volumes}"));
		}
		Ok(())
	}

	#[test]
	fn headshot_band_follows_squat_hull() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, apply_stance_hulls);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
				commands.entity(body).insert(HeadshotBand {
					min_local_y: LocomotionCapsule::HUMANOID.headshot_min_local_y(),
					multiplier: 1.25,
				});
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.update();
		app.world_mut()
			.entity_mut(body)
			.insert(CharacterStance::settled(StanceKind::Squat));
		app.update();
		let band = app.world().get::<HeadshotBand>(body).ok_or_else(|| anyhow!("band"))?;
		let live = app
			.world()
			.get::<LocomotionCapsule>(body)
			.ok_or_else(|| anyhow!("hull"))?
			.headshot_min_local_y();
		if (band.min_local_y - live).abs() > 1e-5 {
			return Err(anyhow!("headshot band should follow the squat hull, not 0.7"));
		}
		if (band.min_local_y - 0.7).abs() < 1e-5 {
			return Err(anyhow!("squat headshot Y must not stay at the standing 0.7 m"));
		}
		Ok(())
	}

	#[test]
	fn restamp_keeps_a_live_squat() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, apply_stance_hulls);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.world_mut()
			.entity_mut(body)
			.insert(CharacterStance::settled(StanceKind::Squat));
		app.update();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
			})
			.map_err(|err| anyhow!("{err}"))?;
		let stance = app.world().get::<CharacterStance>(body).ok_or_else(|| anyhow!("stance"))?;
		if stance.kind != StanceKind::Squat {
			return Err(anyhow!("re-stamp must not stand a live squat"));
		}
		app.update();
		let live = app.world().get::<LocomotionCapsule>(body).ok_or_else(|| anyhow!("hull"))?;
		let squat = LocomotionCapsule::HUMANOID.squat(squat_drop());
		if (live.length - squat.length).abs() > 1e-5 {
			return Err(anyhow!("re-stamp stand hull must be reconciled back to squat"));
		}
		Ok(())
	}

	#[test]
	fn prone_lengthens_terrain_pitch() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, sync_stance_pitch);
		let body = app.world_mut().spawn(Transform::IDENTITY).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				apply_character_controller(&mut commands, body, LocomotionCapsule::HUMANOID);
			})
			.map_err(|err| anyhow!("{err}"))?;
		app.world_mut()
			.entity_mut(body)
			.insert(CharacterStance::settled(StanceKind::Prone));
		let visual = app
			.world_mut()
			.spawn((
				CharacterRoot,
				ChildOf(body),
				TerrainPitch::new(characters::RigSkeletonKind::Humanoid, 0.22, 0.18),
			))
			.id();
		app.update();
		let pitch = app.world().get::<TerrainPitch>(visual).ok_or_else(|| anyhow!("pitch"))?;
		if pitch.pitch_weight < 0.99 {
			return Err(anyhow!("prone should use full pitch weight, got {}", pitch.pitch_weight));
		}
		if pitch.half_span < 0.69 {
			return Err(anyhow!(
				"prone should lengthen the pitch wheelbase, got {}",
				pitch.half_span
			));
		}
		Ok(())
	}

	#[test]
	fn held_squat_drops_more_than_jump_windup() {
		let lengths = LegSegmentLengths::default();
		assert!(
			Squat::held().peak_vertical_drop(lengths)
				> Squat::default().peak_vertical_drop(lengths)
		);
	}
}
