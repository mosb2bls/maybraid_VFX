use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};

/// When the instance snapshots aim. Live transforms only; never writes contacts.
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimSnapshot {
	pub origin: Vec3,
	pub direction: Vec3,
	pub target: Option<Entity>,
	/// Mouth / socket used by [`crate::HazardOrigin::Emission`]. Defaults to
	/// [`Self::origin`].
	pub emission: Vec3,
}

impl AimSnapshot {
	pub fn owner_facing(origin: Vec3, forward: Vec3) -> Self {
		Self {
			origin,
			direction: flatten_direction(forward),
			target: None,
			emission: origin,
		}
	}

	pub fn toward(origin: Vec3, target_point: Vec3, target: Option<Entity>) -> Self {
		Self {
			origin,
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

/// Pick a subject without fabricating visual contacts.
///
/// Order: explicit begin target if it still exists, else a ranked/active
/// [`CombatTargeting`] member, else none (owner facing).
pub fn resolve_target(
	explicit: Option<Entity>,
	targeting: Option<&CombatTargeting>,
	exists: impl Fn(Entity) -> bool,
) -> Option<Entity> {
	if let Some(target) = explicit.filter(|target| exists(*target)) {
		return Some(target);
	}
	let targeting = targeting?;
	if let Some(best) =
		targeting.best().map(|ranked| ranked.entity).filter(|target| exists(*target))
	{
		return Some(best);
	}
	targeting
		.active
		.iter()
		.find(|(entity, member)| member.is_member() && exists(**entity))
		.map(|(entity, _)| *entity)
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
	fn resolve_target_prefers_explicit_alive_entity() {
		let explicit = Entity::from_bits(4);
		let chosen = resolve_target(Some(explicit), None, |entity| entity == explicit);
		assert_eq!(chosen, Some(explicit));
	}

	#[test]
	fn resolve_target_ignores_missing_explicit_and_does_not_invent_contacts() {
		let gone = Entity::from_bits(5);
		let chosen = resolve_target(Some(gone), None, |_| false);
		assert_eq!(chosen, None);
	}

	#[test]
	fn resolve_target_uses_membership_not_contact_memory() -> anyhow::Result<()> {
		let subject = Entity::from_bits(9);
		let mut targeting = CombatTargeting::default();
		targeting.include(subject, TargetSource::OBJECTIVE);
		targeting.rebalance(0.0);
		let chosen = resolve_target(None, Some(&targeting), |entity| entity == subject);
		if chosen != Some(subject) {
			return Err(anyhow::anyhow!("expected objective member, got {chosen:?}"));
		}
		assert!(targeting.contact(subject).is_none());
		Ok(())
	}
}
