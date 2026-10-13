//! Generate budget for Barking's mob channel.

use bevy::prelude::*;

/// Generate budget for [`crate::MobLodChan`].
///
/// Every budget is 16 in the world. Radii, seed, occupancy, and cell size stay
/// constants on the index and stream.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct BarkingConfig {
	pub generate_budget: u32,
}

impl Default for BarkingConfig {
	fn default() -> Self {
		Self { generate_budget: 16 }
	}
}

impl BarkingConfig {
	/// Generate budget 16, the value the world assembler used to insert.
	pub fn world_defaults() -> Self {
		Self { generate_budget: 16 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_defaults_keep_the_sixteen_id_budget() {
		assert_eq!(BarkingConfig::world_defaults().generate_budget, 16);
		assert_eq!(BarkingConfig::default().generate_budget, 16);
	}
}
