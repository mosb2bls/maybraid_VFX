//! Occupied-area tracker with separate furniture vs structure caps.

/// Tracks plan-area consumption against occupancy / structure ceilings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OccupiedBudget {
	pub room_area: f32,
	pub occupied: f32,
	pub furniture_occupancy: f32,
	pub structure_occupancy: f32,
}

impl OccupiedBudget {
	pub fn new(room_area: f32, furniture_occupancy: f32, structure_occupancy: f32) -> Self {
		Self {
			room_area: room_area.max(1e-4),
			occupied: 0.0,
			furniture_occupancy,
			structure_occupancy,
		}
	}

	pub fn ratio(&self) -> f32 {
		self.occupied / self.room_area
	}

	pub fn furniture_full(&self) -> bool {
		self.ratio() >= self.furniture_occupancy
	}

	/// Whether `add` fits under the appropriate cap.
	pub fn accepts(&self, add: f32, is_structure: bool) -> bool {
		let cap = if is_structure { self.structure_occupancy } else { self.furniture_occupancy };
		(self.occupied + add) / self.room_area <= cap + 1e-3
	}

	pub fn commit(&mut self, add: f32) {
		self.occupied += add;
	}
}
