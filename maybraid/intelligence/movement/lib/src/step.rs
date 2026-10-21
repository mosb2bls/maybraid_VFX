//! Interaction-shaped movement steps. Later cores can `From` collider paths into richer enums.

use crate::location::MovementLocation;

/// One step in a movement plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementStep {
	MoveTo(MovementLocation),
}

/// How a plan step drives a [`player::MotorIntent`] NPC slot.
pub trait MovementDrive {
	fn drive_target(&self) -> Option<MovementLocation>;
}

impl MovementDrive for MovementStep {
	fn drive_target(&self) -> Option<MovementLocation> {
		match *self {
			Self::MoveTo(location) => Some(location),
		}
	}
}
