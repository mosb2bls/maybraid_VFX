//! Movement goals. Higher-order systems write these; this crate does not track entities.

use bevy::prelude::Vec2;

use crate::location::MovementLocation;

/// How much [`MovementObjective`] may drift before a higher-order system should
/// insert [`crate::ReplanMovement`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReplanThreshold {
	pub refresh_distance: f32,
	pub check_radius: bool,
	pub check_vantage_weights: bool,
}

/// What the mover is trying to achieve relative to a [`MovementLocation`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementObjective {
	/// Arrive inside the location disk.
	Reach(MovementLocation),
	/// Arrive on the disk boundary (just outside the interior).
	EdgeOf(MovementLocation),
	/// Leave the disk, from inside or through it.
	FleeFrom(MovementLocation),
	/// Take a position that conceals from, and observes, the location.
	VantageOn {
		location: MovementLocation,
		/// Value of concealing the body from the location.
		hide_weight: f32,
		/// Value of a sightline to the location.
		sightline_weight: f32,
	},
}

impl MovementObjective {
	pub fn location(self) -> MovementLocation {
		match self {
			Self::Reach(location) | Self::EdgeOf(location) | Self::FleeFrom(location) => location,
			Self::VantageOn { location, .. } => location,
		}
	}

	pub fn hide_weight(self) -> f32 {
		match self {
			Self::VantageOn { hide_weight, .. } => hide_weight,
			_ => 0.0,
		}
	}

	pub fn sightline_weight(self) -> f32 {
		match self {
			Self::VantageOn { sightline_weight, .. } => sightline_weight,
			_ => 0.0,
		}
	}

	pub fn is_vantage_on(self) -> bool {
		matches!(self, Self::VantageOn { .. })
	}

	/// True when `next` differs enough from `self` to warrant a new plan.
	pub fn needs_replan(self, next: Self, threshold: ReplanThreshold) -> bool {
		if std::mem::discriminant(&self) != std::mem::discriminant(&next) {
			return true;
		}
		if threshold.check_vantage_weights
			&& ((self.hide_weight() - next.hide_weight()).abs() > 0.05
				|| (self.sightline_weight() - next.sightline_weight()).abs() > 0.05)
		{
			return true;
		}
		let a = self.location().point;
		let b = next.location().point;
		if Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z)) >= threshold.refresh_distance
			|| (a.y - b.y).abs() >= threshold.refresh_distance
		{
			return true;
		}
		threshold.check_radius && (self.location().radius - next.location().radius).abs() > 0.05
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::*;

	#[test]
	fn vantage_on_exposes_weights() -> anyhow::Result<()> {
		let objective = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.0),
			hide_weight: 2.0,
			sightline_weight: 3.0,
		};
		assert_eq!(objective.hide_weight(), 2.0);
		assert_eq!(objective.sightline_weight(), 3.0);
		assert_eq!(objective.location().radius, 1.0);
		Ok(())
	}

	#[test]
	fn needs_replan_when_the_watch_point_moves() {
		let threshold = ReplanThreshold {
			refresh_distance: 0.6,
			check_radius: true,
			check_vantage_weights: true,
		};
		let a = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		let near = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 0.2, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		let far = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 2.0, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		assert!(!a.needs_replan(near, threshold));
		assert!(a.needs_replan(far, threshold));
	}

	#[test]
	fn needs_replan_when_vantage_weights_shift() {
		let threshold = ReplanThreshold {
			refresh_distance: 0.6,
			check_radius: true,
			check_vantage_weights: true,
		};
		let seen = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 6.0, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		let hunt = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 6.0, 1.4),
			hide_weight: 3.5,
			sightline_weight: 30.8,
		};
		assert!(seen.needs_replan(hunt, threshold));
	}
}
