use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};

/// When the instance snapshots aim from observed contact knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AimPolicy {
	/// Owner origin and facing at windup start.
	OwnerPoseAtWindup,
	/// Explicit or ranked target position at windup start.
	TargetAtWindup,
	/// Follow the live target through windup; freeze when execute starts.
	TargetAtExecute,
	/// Follow the live target through windup; freeze for execute.
	TrackUntilExecute,
	/// Follow the live target through execute.
	TrackThroughExecute,
}

/// Frozen (or last-known) aim used by telegraphs and hazards.
///
/// `origin` is the owner. `aim_point` is the observed world aim (from
/// [`CombatContact`]), never implied to equal the owner. `emission` is the
/// mouth / socket.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimSnapshot {
	pub origin: Vec3,
	pub aim_point: Vec3,
	pub direction: Vec3,
	pub target: Option<Entity>,
	pub emission: Vec3,
}

impl AimSnapshot {
	pub fn owner_facing(origin: Vec3, forward: Vec3) -> Self {
		Self {
			origin,
			aim_point: origin,
			direction: flatten_direction(forward),
			target: None,
			emission: origin,
		}
	}

	pub fn toward(origin: Vec3, target_point: Vec3, target: Option<Entity>) -> Self {
		Self {
			origin,
			aim_point: target_point,
			direction: flatten_direction(target_point - origin),
			target,
			emission: origin,
		}
	}

	pub fn with_emission(mut self, emission: Vec3) -> Self {
		self.emission = emission;
		self
	}

	pub fn rotation(self) -> Quat {
		Quat::from_rotation_arc(Vec3::Z, self.direction)
	}

	/// Horizontal heading, then an optional pitch override in radians.
	pub fn travel_direction(self, pitch: f32) -> Vec3 {
		let horizontal = flatten_direction(self.direction);
		if pitch.abs() <= 1e-5 {
			return horizontal;
		}
		let right = Vec3::Y.cross(horizontal);
		let right = if right.length_squared() <= 1e-8 { Vec3::X } else { right.normalize() };
		Quat::from_axis_angle(right, pitch) * horizontal
	}

	pub fn tracks(self, policy: AimPolicy, stage: crate::AttackStage) -> bool {
		matches!(
			(policy, stage),
			(
				AimPolicy::TrackThroughExecute,
				crate::AttackStage::Windup | crate::AttackStage::Execute,
			) | (
				AimPolicy::TrackUntilExecute | AimPolicy::TargetAtExecute,
				crate::AttackStage::Windup,
			)
		)
	}
}

/// Observed aim knowledge. Membership alone is not enough to aim.
///
/// Order: contact for an explicit begin target, else [`CombatTargeting::best_contact`].
/// Never reads a live target transform. Missing memory falls back to owner facing.
pub fn resolve_aim_contact(
	explicit: Option<Entity>,
	targeting: Option<&CombatTargeting>,
) -> Option<combat_targeting::CombatContact> {
	let targeting = targeting?;
	if let Some(target) = explicit {
		return targeting.contact(target).copied();
	}
	targeting.best_contact().copied()
}

pub fn owner_point(transform: &Transform, global: Option<&GlobalTransform>) -> Vec3 {
	global.map(GlobalTransform::translation).unwrap_or(transform.translation)
}

pub fn owner_forward(transform: &Transform, global: Option<&GlobalTransform>) -> Vec3 {
	let rotation = global.map(GlobalTransform::rotation).unwrap_or(transform.rotation);
	flatten_direction(rotation * Vec3::NEG_Z)
}

pub fn flatten_direction(direction: Vec3) -> Vec3 {
	let flat = Vec3::new(direction.x, 0.0, direction.z);
	if flat.length_squared() <= 1e-8 {
		Vec3::NEG_Z
	} else {
		flat.normalize()
	}
}

pub fn targeting_allies(targeting: Option<&CombatTargeting>) -> Vec<Entity> {
	let Some(targeting) = targeting else {
		return Vec::new();
	};
	targeting
		.active
		.iter()
		.filter(|(_, member)| member.has_source(TargetSource::ALLY) && member.is_member())
		.map(|(entity, _)| *entity)
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn resolve_aim_contact_requires_memory() {
		let explicit = Entity::from_bits(4);
		assert!(resolve_aim_contact(Some(explicit), None).is_none());
	}

	#[test]
	fn resolve_aim_contact_ignores_membership_without_memory() {
		let subject = Entity::from_bits(5);
		let mut targeting = CombatTargeting::default();
		targeting.include(subject, TargetSource::OBJECTIVE);
		targeting.rebalance(0.0);
		assert!(resolve_aim_contact(Some(subject), Some(&targeting)).is_none());
		assert!(resolve_aim_contact(None, Some(&targeting)).is_none());
		assert!(targeting.contact(subject).is_none());
	}

	#[test]
	fn resolve_aim_contact_uses_observed_aim_point() -> anyhow::Result<()> {
		let subject = Entity::from_bits(9);
		let seen = Vec3::new(4.0, 1.5, -6.0);
		let mut targeting = CombatTargeting::default();
		targeting.upsert_contact(combat_targeting::CombatContact {
			subject,
			position: seen,
			movement_vector: Vec3::ZERO,
			visible_point: seen,
			visible_head: Some(seen + Vec3::Y),
			last_spotted_at: 0.0,
		});
		targeting.rebalance(0.0);
		let Some(contact) = resolve_aim_contact(None, Some(&targeting)) else {
			return Err(anyhow::anyhow!("expected best_contact"));
		};
		if contact.aim_point(false) != seen {
			return Err(anyhow::anyhow!("aim must come from contact, got {:?}", contact.aim_point(false)));
		}
		Ok(())
	}
}
