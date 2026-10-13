//! Kinematic split so overlapping NPC capsules steer apart on XZ.

use bevy::prelude::*;
use characters::LocomotionCapsule;

use crate::body::{CharacterController, MoveWish};
use crate::identity::Npc;
use crate::spawn::CAPSULE_RADIUS;

/// Horizontal distance that starts a kinematic bump between NPC capsules.
///
/// Matches plant occupancy without taking a `poi-intelligence` dependency.
pub const NPC_SEPARATION: f32 = 2.0;

/// Fraction of a unit wish applied at full overlap.
const BUMP_GAIN: f32 = 0.85;
const PARALLEL_ALIGN: f32 = 0.85;

/// Kinematic XZ split between overlapping NPC capsules.
///
/// Added to [`MoveWish`] after drive and before realization so pack-mates
/// start steering apart; capsule contacts still shove if they close.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoftBump {
	pub min_separation: f32,
	pub gain: f32,
}

impl Default for SoftBump {
	fn default() -> Self {
		Self { min_separation: NPC_SEPARATION, gain: BUMP_GAIN }
	}
}

impl SoftBump {
	/// Opposing XZ pushes when `left` and `right` are closer than the pair gap.
	///
	/// Coincident poses use a deterministic heading from the entity bits so the
	/// split does not depend on physics.
	pub fn pair_push(
		self,
		left: Vec2,
		right: Vec2,
		left_radius: f32,
		right_radius: f32,
		left_id: Entity,
		right_id: Entity,
	) -> Option<(Vec2, Vec2)> {
		let min_sep = self.min_separation.max(left_radius + right_radius);
		if min_sep <= 0.0 {
			return None;
		}
		let delta = left - right;
		let dist = delta.length();
		if dist >= min_sep {
			return None;
		}
		let dir = if dist < 1e-4 { split_dir(left_id, right_id) } else { delta / dist };
		let weight = ((min_sep - dist) / min_sep).clamp(0.0, 1.0) * self.gain;
		let push = dir * weight;
		Some((push, -push))
	}

	/// Add `push` onto an XZ wish. A nearly parallel push is rotated 90° so
	/// realization's normalize cannot cancel the split.
	pub fn apply_to_wish(wish: &mut Vec3, push: Vec2) {
		if push.length_squared() < 1e-8 {
			return;
		}
		let heading = Vec2::new(wish.x, wish.z);
		let steer = if heading.length_squared() > 1e-8
			&& heading.normalize().dot(push.normalize()).abs() > PARALLEL_ALIGN
		{
			Vec2::new(-push.y, push.x)
		} else {
			push
		};
		wish.x += steer.x;
		wish.z += steer.y;
	}
}

fn mix_bits(bits: u64) -> u64 {
	let mut z = bits.wrapping_add(0x9E37_79B9_7F4A_7C15);
	z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
	z ^ (z >> 31)
}

fn split_dir(left: Entity, right: Entity) -> Vec2 {
	let bits = mix_bits(left.to_bits() ^ mix_bits(right.to_bits().rotate_left(17)));
	let unit = (bits >> 40) as u32 as f32 / 16_777_216.0;
	Vec2::from_angle(unit * core::f32::consts::TAU)
}

/// Nudge overlapping NPC wishes before [`crate::body::apply_wish_movement`].
pub(crate) fn apply_npc_soft_bump(
	poses: Query<
		(Entity, &Transform, Option<&LocomotionCapsule>),
		(With<Npc>, With<CharacterController>),
	>,
	mut wishes: Query<&mut MoveWish, (With<Npc>, With<CharacterController>)>,
) {
	let mut agents: Vec<(Entity, Vec2, f32)> = poses
		.iter()
		.map(|(entity, transform, hull)| {
			(
				entity,
				transform.translation.xz(),
				hull.map(|hull| hull.radius).unwrap_or(CAPSULE_RADIUS),
			)
		})
		.collect();
	agents.sort_by_key(|(entity, _, _)| entity.to_bits());

	let mut pushes = vec![Vec2::ZERO; agents.len()];
	let bump = SoftBump::default();
	for i in 0..agents.len() {
		for j in (i + 1)..agents.len() {
			let Some((left, right)) = bump.pair_push(
				agents[i].1,
				agents[j].1,
				agents[i].2,
				agents[j].2,
				agents[i].0,
				agents[j].0,
			) else {
				continue;
			};
			pushes[i] += left;
			pushes[j] += right;
		}
	}

	for (i, push) in pushes.into_iter().enumerate() {
		if push.length_squared() < 1e-8 {
			continue;
		}
		let Ok(mut wish) = wishes.get_mut(agents[i].0) else {
			continue;
		};
		SoftBump::apply_to_wish(&mut wish.0, push);
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;
	use crate::identity::Player;

	fn entity(bits: u64) -> Entity {
		Entity::from_bits(bits)
	}

	#[test]
	fn pair_inside_separation_gets_opposing_pushes() {
		let bump = SoftBump::default();
		let Some((left, right)) =
			bump.pair_push(Vec2::ZERO, Vec2::X * 0.5, 0.4, 0.4, entity(1), entity(2))
		else {
			assert!(false, "overlap should produce a bump");
			return;
		};
		assert!(left.dot(right) < 0.0, "opposing {left:?} {right:?}");
		assert!((left + right).length() < 1e-5);
		assert!(left.x < 0.0);
		assert!(right.x > 0.0);
	}

	#[test]
	fn pair_beyond_separation_is_untouched() {
		let bump = SoftBump::default();
		assert!(bump
			.pair_push(Vec2::ZERO, Vec2::X * 5.0, 0.4, 0.4, entity(1), entity(2))
			.is_none());
	}

	#[test]
	fn coincident_pair_uses_entity_bits() {
		let bump = SoftBump::default();
		let first = bump.pair_push(Vec2::ZERO, Vec2::ZERO, 0.4, 0.4, entity(3), entity(9));
		let again = bump.pair_push(Vec2::ZERO, Vec2::ZERO, 0.4, 0.4, entity(3), entity(9));
		let other = bump.pair_push(Vec2::ZERO, Vec2::ZERO, 0.4, 0.4, entity(3), entity(11));
		assert!(first.is_some());
		assert_eq!(first, again);
		assert_ne!(first, other);
		if let Some((left, right)) = first {
			assert!((left + right).length() < 1e-5);
			assert!(left.length() > 1e-4);
		}
	}

	#[test]
	fn parallel_wish_rotates_so_normalize_cannot_cancel() {
		let mut wish = Vec3::X;
		SoftBump::apply_to_wish(&mut wish, Vec2::X * 0.5);
		assert!(wish.z.abs() > 1e-4, "{wish:?}");
		assert!(wish.y.abs() < 1e-6);
	}

	#[test]
	fn overlapping_npcs_get_opposing_lateral_wishes() {
		let mut world = World::new();
		let a = world
			.spawn((
				Npc,
				CharacterController,
				Transform::from_xyz(0.0, 0.0, 0.0),
				MoveWish(Vec3::X),
				LocomotionCapsule::HUMANOID,
			))
			.id();
		let b = world
			.spawn((
				Npc,
				CharacterController,
				Transform::from_xyz(0.5, 0.0, 0.0),
				MoveWish(Vec3::X),
				LocomotionCapsule::HUMANOID,
			))
			.id();
		let player = world
			.spawn((
				Player,
				CharacterController,
				Transform::from_xyz(0.25, 0.0, 0.0),
				MoveWish(Vec3::X),
				LocomotionCapsule::HUMANOID,
			))
			.id();

		assert!(world.run_system_once(apply_npc_soft_bump).is_ok());

		let wish_a = world.get::<MoveWish>(a).map(|wish| wish.0);
		let wish_b = world.get::<MoveWish>(b).map(|wish| wish.0);
		let wish_player = world.get::<MoveWish>(player).map(|wish| wish.0);
		assert!(wish_a.is_some() && wish_b.is_some() && wish_player.is_some());
		if let (Some(wish_a), Some(wish_b), Some(wish_player)) = (wish_a, wish_b, wish_player) {
			assert!((wish_player - Vec3::X).length() < 1e-5, "{wish_player:?}");
			let delta_a = Vec2::new(wish_a.x - 1.0, wish_a.z);
			let delta_b = Vec2::new(wish_b.x - 1.0, wish_b.z);
			assert!(delta_a.length() > 1e-4, "{wish_a:?}");
			assert!(delta_a.dot(delta_b) < 0.0, "opposing {wish_a:?} {wish_b:?}");
			assert!(wish_a.y.abs() < 1e-6 && wish_b.y.abs() < 1e-6);
		}
	}

	#[test]
	fn distant_npcs_keep_their_wish() {
		let mut world = World::new();
		let a = world
			.spawn((
				Npc,
				CharacterController,
				Transform::from_xyz(0.0, 0.0, 0.0),
				MoveWish(Vec3::X),
				LocomotionCapsule::HUMANOID,
			))
			.id();
		let b = world
			.spawn((
				Npc,
				CharacterController,
				Transform::from_xyz(8.0, 0.0, 0.0),
				MoveWish(Vec3::Z),
				LocomotionCapsule::HUMANOID,
			))
			.id();

		assert!(world.run_system_once(apply_npc_soft_bump).is_ok());

		let wish_a = world.get::<MoveWish>(a).map(|wish| wish.0);
		let wish_b = world.get::<MoveWish>(b).map(|wish| wish.0);
		assert_eq!(wish_a, Some(Vec3::X));
		assert_eq!(wish_b, Some(Vec3::Z));
	}
}
