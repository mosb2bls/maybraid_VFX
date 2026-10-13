//! Bounded per-account charge and the result of a signed mutation.

use crate::identity::sanitize_capacity;

/// Current charge clamped to `[0, capacity]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LubikanCharge {
	current: f32,
	capacity: f32,
}

impl LubikanCharge {
	/// Empty charge at `capacity`.
	pub fn empty(capacity: f32) -> Self {
		let capacity = sanitize_capacity(capacity);
		Self { current: 0.0, capacity }
	}

	/// Charge at `current`, clamped into a sanitized capacity.
	pub fn new(current: f32, capacity: f32) -> Self {
		let capacity = sanitize_capacity(capacity);
		let current = if current.is_finite() { current.clamp(0.0, capacity) } else { 0.0 };
		Self { current, capacity }
	}

	pub fn current(self) -> f32 {
		self.current
	}

	pub fn capacity(self) -> f32 {
		self.capacity
	}

	pub fn fraction(self) -> f32 {
		if self.capacity <= 0.0 {
			0.0
		} else {
			(self.current / self.capacity).clamp(0.0, 1.0)
		}
	}

	pub fn can_afford(self, cost: f32) -> bool {
		cost.is_finite() && cost >= 0.0 && self.current + 1e-5 >= cost
	}

	/// Resize capacity and clamp the current value into the new range.
	pub fn set_capacity(&mut self, capacity: f32) {
		self.capacity = sanitize_capacity(capacity);
		self.current = self.current.clamp(0.0, self.capacity);
	}

	/// Apply a signed delta. Non-finite requests become `0`. The result reports
	/// the requested amount, the applied amount, the new value, and which
	/// bounds clamped the write.
	pub fn apply(&mut self, delta: f32) -> LubikanChargeMutation {
		let requested = if delta.is_finite() { delta } else { 0.0 };
		let before = self.current;
		let unclamped = before + requested;
		let new_value = unclamped.clamp(0.0, self.capacity);
		self.current = new_value;
		LubikanChargeMutation {
			requested,
			applied: new_value - before,
			new_value,
			clamped_floor: unclamped < 0.0,
			clamped_capacity: unclamped > self.capacity,
		}
	}
}

/// Result of [`LubikanCharge::apply`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LubikanChargeMutation {
	pub requested: f32,
	pub applied: f32,
	pub new_value: f32,
	pub clamped_floor: bool,
	pub clamped_capacity: bool,
}

impl LubikanChargeMutation {
	pub fn changed(self) -> bool {
		self.applied != 0.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn apply_clamps_to_floor_and_capacity() {
		let mut charge = LubikanCharge::new(10.0, 20.0);
		let high = charge.apply(100.0);
		assert_eq!(high.requested, 100.0);
		assert_eq!(high.applied, 10.0);
		assert_eq!(high.new_value, 20.0);
		assert!(high.clamped_capacity);
		assert!(!high.clamped_floor);
		let low = charge.apply(-100.0);
		assert_eq!(low.applied, -20.0);
		assert_eq!(low.new_value, 0.0);
		assert!(low.clamped_floor);
		assert!(!low.clamped_capacity);
	}

	#[test]
	fn non_finite_request_does_not_move_charge() {
		let mut charge = LubikanCharge::new(4.0, 10.0);
		let mutation = charge.apply(f32::NAN);
		assert_eq!(mutation.requested, 0.0);
		assert_eq!(mutation.applied, 0.0);
		assert_eq!(charge.current(), 4.0);
		assert!(!mutation.clamped_floor);
		assert!(!mutation.clamped_capacity);
	}

	#[test]
	fn affordance_rejects_insufficient_and_invalid_costs() {
		let charge = LubikanCharge::new(5.0, 10.0);
		assert!(charge.can_afford(5.0));
		assert!(!charge.can_afford(5.1));
		assert!(!charge.can_afford(-1.0));
		assert!(!charge.can_afford(f32::NAN));
	}
}
