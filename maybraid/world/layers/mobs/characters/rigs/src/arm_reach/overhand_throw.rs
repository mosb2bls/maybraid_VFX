//! Overhand throw reach profile (primed chest-high hold → cock → snap → recover).

use bevy::prelude::*;

use super::{ArmReachClamp, ArmReachPole, ArmReachTrack};

/// Right-arm overhand throw overlay authored in body space (+X right, +Y up, +Z forward).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverhandThrow;

impl OverhandThrow {
	const KEYFRAMES: [(f32, Vec3); 6] = [
		(0.00, Vec3::new(0.10, 0.14, 0.42)),
		(0.22, Vec3::new(0.16, 0.22, 0.28)),
		(0.45, Vec3::new(0.22, 0.36, 0.16)),
		(0.61, Vec3::new(0.08, 0.18, 0.46)),
		(0.82, Vec3::new(0.08, 0.02, 0.42)),
		(1.00, Vec3::new(0.10, 0.10, 0.38)),
	];

	const CLAMP: ArmReachClamp = ArmReachClamp { min_x: Some(0.06), min_z: Some(0.08) };

	/// Lateral pole so the right elbow wings out instead of folding through the torso.
	pub fn pole() -> ArmReachPole {
		ArmReachPole::new(Vec3::new(1.0, 0.45, -0.2), Vec3::new(0.6, 0.8, 0.2))
	}

	pub fn track() -> ArmReachTrack {
		ArmReachTrack::from_keyframes(&Self::KEYFRAMES)
	}

	/// Sample the throw reach at normalized phase `t`, with safety clamp applied.
	pub fn reach_at(t: f32) -> Vec3 {
		Self::CLAMP.apply(Self::track().sample(t))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn primed_hold_is_high_and_in_front() -> anyhow::Result<()> {
		let primed = OverhandThrow::reach_at(0.0);
		assert!(
			primed.x > 0.06 && primed.x < 0.16,
			"primed hold stays slightly out, got {primed:?}"
		);
		assert!(primed.y > 0.0, "primed hold is chest-high, got {primed:?}");
		assert!(primed.z > 0.36, "primed hold is in front, got {primed:?}");
		Ok(())
	}

	#[test]
	fn reach_stays_lateral_and_forward() -> anyhow::Result<()> {
		for t in [0.0, 0.22, 0.45, 0.61, 0.82, 1.0] {
			let reach = OverhandThrow::reach_at(t);
			assert!(reach.x > 0.0, "t={t} crossed the body, got {reach:?}");
			assert!(reach.z > 0.0, "t={t} went behind, got {reach:?}");
		}
		Ok(())
	}

	#[test]
	fn swing_raises_then_snaps_forward() -> anyhow::Result<()> {
		let start = OverhandThrow::track().sample(0.0);
		let cock = OverhandThrow::track().sample(0.45);
		let release = OverhandThrow::track().sample(0.61);
		assert!(cock.y > start.y + 0.15, "must raise, start={start:?} cock={cock:?}");
		assert!(release.z > cock.z + 0.2, "must snap forward, cock={cock:?} release={release:?}");
		Ok(())
	}
}
