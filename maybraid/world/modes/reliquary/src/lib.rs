//! Reliquary: a home-row session that is not wired yet.

use bevy::prelude::*;

pub const LABEL: &str = "Reliquary";

/// What selecting Reliquary does. It still does not open a world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliquaryRoute {
	Unimplemented,
}

pub fn route() -> ReliquaryRoute {
	ReliquaryRoute::Unimplemented
}

/// Marker so the executable loads this mode beside the others.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ReliquaryMode;

pub struct ReliquaryPlugin;

impl Plugin for ReliquaryPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ReliquaryMode>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reliquary_is_unimplemented() {
		assert_eq!(route(), ReliquaryRoute::Unimplemented);
		assert_eq!(LABEL, "Reliquary");
	}
}
