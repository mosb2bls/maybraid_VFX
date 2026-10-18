//! Firearm combat brain: who to shoot, how to aim, when to fire.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use characters::{CharacterHeading, CharacterRoot};
use combat_targeting::{CombatContact, CombatTargeting};
use firearm_user::FirearmUser;
use firearms::{
	muzzle_world, BoneMap, Cadence, FireControl, FirearmMembers, ProjectileLoad, RigRoot, Weapon,
	WeaponFired, WeaponTrigger,
};
use movement_intelligence::{MovementBody, MovementIntelligence};
use player::{clamp_aim_pitch, look_delta, PlayerLook, PlayerYawOwner};
use spotting_intelligence::{SpotBounds, SpotSubject};

use crate::engagement::{allows_fire, FirearmEngagement};
use crate::targeting::FirearmTargeting;

type FirearmAimControl<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static Transform,
		&'static MovementIntelligence,
		&'static FirearmUser,
		&'static mut FirearmIntelligence,
		&'static mut CombatTargeting,
		&'static FirearmTargeting,
		&'static mut PlayerLook,
	),
>;

type FirearmFireControl<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static mut FirearmIntelligence,
		&'static FirearmUser,
		&'static CombatTargeting,
		&'static FirearmTargeting,
		Option<&'static FirearmEngagement>,
	),
>;

#[derive(SystemParam)]
pub(crate) struct FirearmTriggerControl<'w, 's> {
	weapons: Query<'w, 's, (&'static Weapon, Option<&'static FireControl>)>,
	triggers: Query<'w, 's, &'static mut WeaponTrigger>,
}

/// How a firearm combatant aims and stays on a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirearmIntelligenceSettings {
	/// 0..=1. 1 is a tight aim cone and a high fire threshold.
	pub accuracy: f32,
	/// Maximum angular travel toward the desired target, in radians per second.
	pub tracking_rate: f32,
	/// 0..=1. Higher skill reduces how far aim trails a moving target.
	pub motion_tracking: f32,
	/// 0..=1. Higher skill reacts sooner and clears recoil error faster.
	pub counter_recoil: f32,
	/// Seconds an acquired trigger may stay held while recoil takes the bore off target.
	pub alignment_grace: f32,
	/// Prefer the remembered visible head sample when the shooter is going for one.
	pub headshots: f32,
	/// Sightline rays traced each frame, shared across ranked candidates.
	pub vision: u16,
	/// 0..=1. Stick to the current target versus the nearest. Also spends more
	/// of [`Self::vision`] on the highest-ranked target.
	pub focus: f32,
	/// 0..=1. How quickly they pull the trigger once the bore is on target.
	/// The weapon interval is the rate of fire after that; this is not a second
	/// cadence. 0 never fires. 1 fires as soon as aligned.
	pub trigger_happiness: f32,
	/// 0..=1. Willingness to fire through an obstructed line of fire.
	pub wall_firing: f32,
	/// Seconds a last-known observation remains actionable for look and hunt.
	pub target_spotting_memory: f32,
	/// Seconds since the last clear sightline required to actually fire.
	/// Look may track a remembered pose; fire needs a current hole to shoot through.
	pub fire_spotting_freshness: f32,
}

impl Default for FirearmIntelligenceSettings {
	fn default() -> Self {
		Self {
			accuracy: 0.75,
			tracking_rate: 6.0,
			motion_tracking: 0.6,
			counter_recoil: 0.6,
			alignment_grace: 0.08,
			headshots: 0.15,
			vision: 9,
			focus: 0.6,
			trigger_happiness: 0.45,
			wall_firing: 0.0,
			target_spotting_memory: 2.5,
			fire_spotting_freshness: 0.2,
		}
	}
}

/// Per-user firearm aim and trigger state.
#[derive(Component, Debug, Clone)]
pub struct FirearmIntelligence {
	pub settings: FirearmIntelligenceSettings,
	aiming_head: bool,
	next_aim_choice_at: f32,
	next_trigger_at: f32,
	on_target: bool,
	last_aligned_at: f32,
	tracked_look: Vec2,
	recoil_offset: Vec2,
	last_output_look: Vec2,
	aim_initialized: bool,
	counter_recoil_ready_at: f32,
}

impl FirearmIntelligence {
	pub fn new() -> Self {
		Self {
			settings: FirearmIntelligenceSettings::default(),
			aiming_head: false,
			next_aim_choice_at: 0.0,
			next_trigger_at: 0.0,
			on_target: false,
			last_aligned_at: f32::NEG_INFINITY,
			tracked_look: Vec2::ZERO,
			recoil_offset: Vec2::ZERO,
			last_output_look: Vec2::ZERO,
			aim_initialized: false,
			counter_recoil_ready_at: 0.0,
		}
	}

	fn realize_aim(&mut self, look: &mut PlayerLook, desired: Option<Vec2>, now: f32, dt: f32) {
		let observed = Vec2::new(look.yaw, look.pitch);
		if !self.aim_initialized {
			self.tracked_look = observed;
			self.last_output_look = observed;
			self.aim_initialized = true;
		} else {
			self.recoil_offset += look_delta(self.last_output_look, observed);
		}

		if let Some(desired) = desired {
			self.tracked_look = Self::move_look_towards(
				self.tracked_look,
				desired,
				self.settings.tracking_rate.max(0.0) * dt.max(0.0),
			);
		}

		let recovery_dt = (now - self.counter_recoil_ready_at).clamp(0.0, dt.max(0.0));
		self.recoil_offset = self.settings.recover_recoil(self.recoil_offset, recovery_dt);

		let output = Vec2::new(
			self.tracked_look.x + self.recoil_offset.x,
			clamp_aim_pitch(self.tracked_look.y + self.recoil_offset.y),
		);
		look.yaw = output.x;
		look.pitch = output.y;
		self.last_output_look = output;
	}

	fn move_look_towards(current: Vec2, target: Vec2, max_step: f32) -> Vec2 {
		let delta = look_delta(current, target);
		let distance = delta.length();
		if distance <= max_step || distance <= 1e-6 {
			return current + delta;
		}
		current + delta * (max_step.max(0.0) / distance)
	}
}

impl FirearmIntelligenceSettings {
	fn motion_tracking_delay(&self) -> f32 {
		let skill = self.motion_tracking.clamp(0.0, 1.0);
		0.3 + (0.03 - 0.3) * skill
	}

	pub(crate) fn perceive_motion(&self, position: Vec3, velocity: Vec3) -> Vec3 {
		position - velocity * self.motion_tracking_delay()
	}

	fn counter_recoil_delay(&self) -> f32 {
		let skill = self.counter_recoil.clamp(0.0, 1.0);
		0.15 + (0.025 - 0.15) * skill
	}

	fn counter_recoil_half_life(&self) -> f32 {
		let skill = self.counter_recoil.clamp(0.0, 1.0);
		0.3 + (0.05 - 0.3) * skill * skill
	}

	fn recover_recoil(&self, offset: Vec2, dt: f32) -> Vec2 {
		if dt <= 0.0 {
			return offset;
		}
		offset * (-dt / self.counter_recoil_half_life()).exp2()
	}

	fn acquire_delay(&self) -> f32 {
		0.45 * (1.0 - self.trigger_happiness.clamp(0.0, 1.0))
	}

	/// First shot needs a bore on the capsule. An acquired shot may remain held for
	/// a bounded grace period while recoil carries the bore off target.
	fn hold_trigger(
		&self,
		aligned: bool,
		within_alignment_grace: bool,
		obstruction_allowed: bool,
	) -> bool {
		if self.trigger_happiness <= 0.0 || !obstruction_allowed {
			return false;
		}
		aligned || within_alignment_grace
	}

	/// Cosine of the allowed bore error. Tightens with range so a passing shot can
	/// actually hit the capsule; `accuracy` adds a little extra miss.
	fn fire_alignment(&self, distance: f32, radius: f32) -> f32 {
		let hit = (radius.max(0.05) / distance.max(0.2)).atan();
		let slack = (1.0 - self.accuracy.clamp(0.0, 1.0)) * 0.05 + 0.012;
		(hit + slack).clamp(0.01, 0.4).cos()
	}

	fn willing_to_fire_through_wall(&self, entity: Entity, now: f32) -> bool {
		let willingness = self.wall_firing.clamp(0.0, 1.0);
		if willingness <= 0.0 {
			return false;
		}
		if willingness >= 1.0 {
			return true;
		}
		Self::frac_noise(entity.to_bits() as f32 * 0.017 + now.floor() * 7.13) < willingness
	}

	fn look_angles(&self, to: Vec3, entity: Entity, elapsed: f32) -> (f32, f32) {
		let cone = (1.0 - self.accuracy.clamp(0.0, 1.0)) * 0.12;
		let shake = Self::jitter(entity, elapsed) * cone;
		let yaw = (-to.x).atan2(-to.z) + shake.x;
		let xz = Vec2::new(to.x, to.z).length();
		let pitch = to.y.atan2(xz.max(1e-4)) + shake.y;
		(yaw, clamp_aim_pitch(pitch))
	}

	fn jitter(entity: Entity, elapsed: f32) -> Vec2 {
		let seed = entity.to_bits() as f32 * 0.013 + elapsed.floor();
		Vec2::new(Self::frac_noise(seed), Self::frac_noise(seed * 1.37)) * 2.0 - Vec2::ONE
	}

	fn frac_noise(x: f32) -> f32 {
		(x.sin() * 43_758.547).fract().abs()
	}
}

impl Default for FirearmIntelligence {
	fn default() -> Self {
		Self::new()
	}
}

/// Select a remembered target and turn the user's desired look toward it.
///
/// Look is solved from the stock (the pose pivot), not the muzzle. The gun
/// rotates about the shoulder; aiming from the barrel tip pitches the bore high.
pub(crate) fn aim_at_firearm_targets(
	time: Res<Time>,
	mut combatants: FirearmAimControl,
	guns: Query<&FirearmMembers>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	bodies: Query<&Transform, Without<FirearmIntelligence>>,
) {
	let elapsed = time.elapsed_secs();
	let dt = time.delta_secs();
	for (
		entity,
		transform,
		movement,
		user,
		mut brain,
		mut targeting,
		firearm_targeting,
		mut look,
	) in &mut combatants
	{
		let from = aim_pivot(user.held, transform.translation, movement, &guns, &maps, &globals);
		let desired = targeting.best_contact().copied().map(|target| {
			if targeting.engaged != Some(target.subject) || elapsed >= brain.next_aim_choice_at {
				brain.aiming_head = FirearmIntelligenceSettings::frac_noise(
					entity.to_bits() as f32 * 0.013
						+ target.subject.to_bits() as f32 * 0.019
						+ elapsed.floor(),
				) < brain.settings.headshots.clamp(0.0, 1.0);
				brain.next_aim_choice_at = elapsed + 1.5;
			}
			targeting.engage(target.subject);
			let current = bodies.get(target.subject).ok().map(|transform| transform.translation);
			let remembered = target.aim_point(brain.aiming_head);
			let aim_at =
				current.map_or(remembered, |position| remembered + (position - target.position));
			let perceived =
				firearm_targeting.select(target.subject, brain.aiming_head, false).map_or_else(
					|| brain.settings.perceive_motion(aim_at, target.movement_vector),
					|trajectory| trajectory.aim_point,
				);
			let to = perceived - from;
			let (yaw, pitch) = brain.settings.look_angles(to, entity, elapsed);
			Vec2::new(yaw, pitch)
		});
		if desired.is_none() {
			targeting.clear_engagement();
		}
		brain.realize_aim(&mut look, desired, elapsed, dt);
	}
}

/// Record the shot-time reaction window separately from the recoil path. The
/// actual angular displacement is observed from [`PlayerLook`] on the next aim tick.
pub(crate) fn note_weapon_recoil(
	time: Res<Time>,
	mut fired: MessageReader<WeaponFired>,
	mut combatants: Query<&mut FirearmIntelligence>,
) {
	let now = time.elapsed_secs();
	for event in fired.read() {
		if event.recoil <= 0.0 {
			continue;
		}
		if let Ok(mut brain) = combatants.get_mut(event.shooter) {
			brain.counter_recoil_ready_at = now + brain.settings.counter_recoil_delay();
		}
	}
}

/// Turn the visual body toward combat look before the held-firearm pose applies
/// its local yaw cone.
pub(crate) fn orient_firearm_combatants(
	time: Res<Time>,
	combatants: Query<(&PlayerLook, &FirearmIntelligence, &CombatTargeting)>,
	mut visuals: Query<
		(&ChildOf, &mut Transform, &mut CharacterHeading, Option<&mut PlayerYawOwner>),
		With<CharacterRoot>,
	>,
) {
	let amount = (time.delta_secs() * 5.0).clamp(0.0, 1.0);
	for (child_of, mut visual, mut heading, yaw_owner) in &mut visuals {
		let Ok((look, _, targeting)) = combatants.get(child_of.parent()) else {
			continue;
		};
		if targeting.engaged.is_none() {
			if let Some(mut yaw_owner) = yaw_owner {
				if *yaw_owner != PlayerYawOwner::Wish {
					*yaw_owner = PlayerYawOwner::Wish;
				}
			}
			continue;
		}
		if let Some(mut yaw_owner) = yaw_owner {
			if *yaw_owner != PlayerYawOwner::Look {
				*yaw_owner = PlayerYawOwner::Look;
			}
		}
		let forward = Quat::from_axis_angle(Vec3::Y, look.yaw) * -Vec3::Z;
		if !heading.would_turn_toward(&visual, forward) {
			continue;
		}
		let current = heading.facing(&visual);
		heading.set(&mut visual, current.slerp(forward, amount));
	}
}

/// Hold the trigger when the posed bore is on a freshly spotted point and the
/// obstruction policy allows the shot.
pub(crate) fn fire_at_spotted_targets(
	time: Res<Time>,
	mut combatants: FirearmFireControl,
	guns: Query<&FirearmMembers>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	subjects: Query<&SpotSubject>,
	mut weapon_control: FirearmTriggerControl,
) {
	let now = time.elapsed_secs();
	for (entity, mut brain, user, targeting, firearm_targeting, engagement) in &mut combatants {
		let target = engaged_target(targeting).copied();
		let Some(target) = target else {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		};
		if !allows_fire(engagement, target.subject) {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		};
		let Ok((weapon, control)) = weapon_control.weapons.get(user.held) else {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		};
		let Some(global) = gun_landmark(user.held, "barrel", &guns, &maps, &globals) else {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		};
		let (muzzle, bore) = muzzle_world(global);
		let fresh = target.is_fresh(now, brain.settings.fire_spotting_freshness);
		let allow_blocked = brain.settings.willing_to_fire_through_wall(entity, now);
		let Some(trajectory) =
			firearm_targeting.select(target.subject, brain.aiming_head, allow_blocked)
		else {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		};
		if !fresh || trajectory.distance <= 1e-4 {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		}
		let delta = trajectory.aim_point - muzzle;
		let distance = delta.length();
		let desired = delta / distance;
		let radius =
			subjects.get(target.subject).ok().map_or(0.4, |subject| match subject.bounds {
				SpotBounds::Capsule { radius, .. } => radius,
			});
		let aligned = bore.dot(desired) >= brain.settings.fire_alignment(distance, radius);
		if aligned {
			brain.last_aligned_at = now;
		}
		let alignment_grace =
			effective_alignment_grace(brain.settings.alignment_grace, weapon, control);
		let within_alignment_grace =
			brain.on_target && now - brain.last_aligned_at <= alignment_grace;
		if !brain.settings.hold_trigger(
			aligned,
			within_alignment_grace,
			trajectory.clear || allow_blocked,
		) {
			brain.on_target = false;
			set_trigger(user, false, &mut weapon_control.triggers);
			continue;
		}
		if !brain.on_target {
			brain.on_target = true;
			brain.next_trigger_at = now + brain.settings.acquire_delay();
		}
		set_cadenced_trigger(
			user,
			weapon,
			control,
			now >= brain.next_trigger_at,
			&mut weapon_control.triggers,
		);
	}
}

fn engaged_target(targeting: &CombatTargeting) -> Option<&CombatContact> {
	let engaged = targeting.engaged?;
	targeting.contact(engaged)
}

pub(crate) fn gun_landmark<'a>(
	held: Entity,
	name: &str,
	guns: &Query<&FirearmMembers>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &'a Query<&GlobalTransform>,
) -> Option<&'a GlobalTransform> {
	let members = guns.get(held).ok()?;
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = globals.get(entity) {
			return Some(global);
		}
	}
	None
}

fn aim_pivot(
	held: Entity,
	body: Vec3,
	movement: &MovementIntelligence,
	guns: &Query<&FirearmMembers>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &Query<&GlobalTransform>,
) -> Vec3 {
	if let Some(stock) = gun_landmark(held, "stock", guns, maps, globals) {
		return stock.translation();
	}
	if let Ok(root) = globals.get(held) {
		return root.translation();
	}
	movement.ability.eye_point(body)
}

fn set_trigger(user: &FirearmUser, fire: bool, triggers: &mut Query<&mut WeaponTrigger>) {
	if let Ok(mut trigger) = triggers.get_mut(user.held) {
		trigger.0 = fire;
	}
}

fn set_cadenced_trigger(
	user: &FirearmUser,
	weapon: &Weapon,
	control: Option<&FireControl>,
	acquired: bool,
	triggers: &mut Query<&mut WeaponTrigger>,
) {
	let Ok(mut trigger) = triggers.get_mut(user.held) else {
		return;
	};
	trigger.0 = acquired && next_trigger_level(weapon, control, trigger.0);
}

fn next_trigger_level(
	weapon: &Weapon,
	control: Option<&FireControl>,
	currently_held: bool,
) -> bool {
	if matches!(weapon.load, ProjectileLoad::Laser(_)) {
		return true;
	}
	match control.map(|control| control.cadence).unwrap_or(Cadence::Auto) {
		Cadence::Auto | Cadence::Gated => true,
		Cadence::Semi => !currently_held,
		Cadence::Burst => control.is_some_and(|control| control.burst_left > 0),
	}
}

fn effective_alignment_grace(
	configured: f32,
	weapon: &Weapon,
	control: Option<&FireControl>,
) -> f32 {
	let cadence_floor = if matches!(weapon.load, ProjectileLoad::Laser(_)) {
		0.25
	} else {
		match control.map(|control| control.cadence).unwrap_or(Cadence::Auto) {
			Cadence::Auto | Cadence::Gated => 0.2,
			Cadence::Burst => 0.12,
			Cadence::Semi => 0.0,
		}
	};
	configured.max(cadence_floor).max(0.0)
}

#[cfg(test)]
fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
	Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_rotation_x(pitch) * -Vec3::Z
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn look_dir_matches_look_angles_without_jitter() -> anyhow::Result<()> {
		let to = Vec3::new(3.0, 1.0, -4.0);
		let settings = FirearmIntelligenceSettings { accuracy: 1.0, ..Default::default() };
		let (yaw, pitch) = settings.look_angles(to, Entity::from_bits(1), 0.0);
		let aimed = look_dir(yaw, pitch);
		let expected = to.normalize();
		assert!(aimed.dot(expected) > 0.999, "{aimed} vs {expected}");
		Ok(())
	}

	#[test]
	fn trigger_happiness_shortens_acquire_delay() {
		let eager = FirearmIntelligenceSettings { trigger_happiness: 1.0, ..Default::default() };
		let hesitant = FirearmIntelligenceSettings { trigger_happiness: 0.0, ..Default::default() };
		assert!(eager.acquire_delay() < hesitant.acquire_delay());
		assert!(eager.acquire_delay() < 1e-4);
	}

	#[test]
	fn tracking_rate_limits_aim_travel() {
		let mut brain = FirearmIntelligence::new();
		brain.settings.tracking_rate = 1.0;
		let mut look = PlayerLook::default();
		brain.realize_aim(&mut look, Some(Vec2::new(1.0, 0.0)), 0.1, 0.1);
		assert!((look.yaw - 0.1).abs() < 1e-5);
	}

	#[test]
	fn motion_tracking_skill_reduces_perception_delay() {
		let poor_settings =
			FirearmIntelligenceSettings { motion_tracking: 0.0, ..Default::default() };
		let skilled_settings =
			FirearmIntelligenceSettings { motion_tracking: 1.0, ..Default::default() };
		assert!((poor_settings.motion_tracking_delay() - 0.3).abs() < 1e-5);
		assert!((skilled_settings.motion_tracking_delay() - 0.03).abs() < 1e-5);

		let position = Vec3::new(2.0, 0.0, 0.0);
		let velocity = Vec3::new(4.0, 0.0, 0.0);
		let poor = poor_settings.perceive_motion(position, velocity);
		let skilled = skilled_settings.perceive_motion(position, velocity);
		assert!(poor.x < skilled.x);
		assert!(skilled.x < position.x);
	}

	#[test]
	fn look_tracking_takes_the_short_way_across_pi() {
		let current = Vec2::new(std::f32::consts::PI - 0.05, 0.0);
		let target = Vec2::new(-std::f32::consts::PI + 0.05, 0.0);
		let moved = FirearmIntelligence::move_look_towards(current, target, 0.04);
		assert!((moved.x - current.x - 0.04).abs() < 1e-5);
	}

	#[test]
	fn better_counter_recoil_recovers_faster() {
		let offset = Vec2::new(0.08, 0.08);
		let poor = FirearmIntelligenceSettings { counter_recoil: 0.0, ..Default::default() };
		let skilled = FirearmIntelligenceSettings { counter_recoil: 1.0, ..Default::default() };
		let poor_offset = poor.recover_recoil(offset, 0.05);
		let skilled_offset = skilled.recover_recoil(offset, 0.05);
		assert!(skilled_offset.length() < poor_offset.length());
		assert!((skilled_offset.length() - offset.length() * 0.5).abs() < 1e-5);
	}

	#[test]
	fn recoil_displacement_survives_the_next_aim_tick() {
		let mut brain = FirearmIntelligence::new();
		brain.counter_recoil_ready_at = 1.0;
		let mut look = PlayerLook::default();
		brain.realize_aim(&mut look, Some(Vec2::ZERO), 0.0, 0.0);
		look.yaw += 0.04;
		look.pitch += 0.08;
		brain.realize_aim(&mut look, Some(Vec2::ZERO), 0.02, 0.02);
		assert!((look.yaw - 0.04).abs() < 1e-5);
		assert!((look.pitch - 0.08).abs() < 1e-5);
	}

	#[test]
	fn fire_alignment_tightens_with_range() {
		let settings = FirearmIntelligenceSettings { accuracy: 1.0, ..Default::default() };
		let close = settings.fire_alignment(2.0, 0.4);
		let far = settings.fire_alignment(20.0, 0.4);
		assert!(far > close, "{far} vs {close}");
		assert!(far > 0.99, "{far}");
	}

	#[test]
	fn right_offset_pivot_aims_left_of_eye() {
		let settings = FirearmIntelligenceSettings { accuracy: 1.0, ..Default::default() };
		let target = Vec3::new(0.0, 1.0, -10.0);
		let (eye_yaw, _) =
			settings.look_angles(target - Vec3::new(0.0, 1.0, 0.0), Entity::from_bits(1), 0.0);
		let (stock_yaw, _) =
			settings.look_angles(target - Vec3::new(0.3, 1.0, 0.0), Entity::from_bits(1), 0.0);
		assert!(stock_yaw > eye_yaw, "{stock_yaw} vs {eye_yaw}");
	}

	#[test]
	fn raised_muzzle_is_not_the_pose_pivot() {
		let settings = FirearmIntelligenceSettings { accuracy: 1.0, ..Default::default() };
		let target = Vec3::new(0.0, 1.05, -8.0);
		let stock = Vec3::new(0.25, 1.35, 0.0);
		let muzzle = Vec3::new(0.25, 1.55, -0.55);
		let (stock_yaw, stock_pitch) =
			settings.look_angles(target - stock, Entity::from_bits(1), 0.0);
		let (muzzle_yaw, muzzle_pitch) =
			settings.look_angles(target - muzzle, Entity::from_bits(1), 0.0);
		let stock_dir = look_dir(stock_yaw, stock_pitch);
		let muzzle_dir = look_dir(muzzle_yaw, muzzle_pitch);
		assert!(
			(stock_dir - muzzle_dir).length() > 0.01,
			"stock {stock_dir} vs muzzle {muzzle_dir}"
		);
	}

	#[test]
	fn zero_wall_firing_rejects_obstructions() {
		let none = FirearmIntelligenceSettings { wall_firing: 0.0, ..Default::default() };
		let all = FirearmIntelligenceSettings { wall_firing: 1.0, ..Default::default() };
		assert!(!none.willing_to_fire_through_wall(Entity::from_bits(1), 0.0));
		assert!(all.willing_to_fire_through_wall(Entity::from_bits(1), 0.0));
	}

	#[test]
	fn hold_trigger_only_keeps_a_lock_inside_alignment_grace() {
		let settings = FirearmIntelligenceSettings { trigger_happiness: 0.9, ..Default::default() };
		assert!(settings.hold_trigger(false, true, true));
		assert!(!settings.hold_trigger(false, false, true));
		assert!(!settings.hold_trigger(true, true, false));
		let never = FirearmIntelligenceSettings { trigger_happiness: 0.0, ..Default::default() };
		assert!(!never.hold_trigger(true, true, true));
	}

	#[test]
	fn sustained_weapons_keep_the_trigger_held() {
		let laser = Weapon::laser();
		let ballistic = Weapon::bullet();
		let automatic = FireControl::auto();
		let gated = FireControl::gated();
		assert!(next_trigger_level(&laser, Some(&automatic), true));
		assert!(next_trigger_level(&ballistic, Some(&automatic), true));
		assert!(next_trigger_level(&ballistic, Some(&gated), true));
		assert!(effective_alignment_grace(0.08, &laser, Some(&automatic)) >= 0.25);
		assert!(effective_alignment_grace(0.08, &ballistic, Some(&automatic)) >= 0.2);
	}

	#[test]
	fn discrete_weapons_release_between_sequences() {
		let weapon = Weapon::bullet();
		let semi = FireControl::semi();
		assert!(next_trigger_level(&weapon, Some(&semi), false));
		assert!(!next_trigger_level(&weapon, Some(&semi), true));

		let mut burst = FireControl::burst(2);
		assert!(next_trigger_level(&weapon, Some(&burst), true));
		burst.note_shot();
		burst.note_shot();
		assert!(!next_trigger_level(&weapon, Some(&burst), true));
	}

	#[test]
	fn fresh_sight_requires_a_recent_combat_observation() {
		let entity = Entity::from_bits(1);
		let contact = CombatContact {
			subject: entity,
			position: Vec3::ZERO,
			visible_point: Vec3::ZERO,
			visible_head: None,
			movement_vector: Vec3::ZERO,
			last_spotted_at: 0.9,
		};
		assert!(contact.is_fresh(1.0, 0.2));
		assert!(!contact.is_fresh(1.5, 0.2));
	}
}
