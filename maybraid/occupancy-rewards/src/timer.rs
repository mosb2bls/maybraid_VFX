//! Per-actor interval timers. Progress is integer microseconds so exact
//! boundaries and large `dt` steps stay deterministic.

use std::time::Duration;

use crate::identity::OccupancyMatchKey;
use crate::rule::RuleId;
use crate::tag::ConstructionTag;

/// One running or persisted occupancy interval.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OccupancyTimer {
	pub accumulated_us: u128,
	pub awards_emitted: u64,
}

impl OccupancyTimer {
	pub fn tick(&mut self, dt: Duration, interval: Duration) -> u64 {
		self.accumulated_us = self.accumulated_us.saturating_add(dt.as_micros());
		let interval_us = interval.as_micros().max(1);
		let total = (self.accumulated_us / interval_us) as u64;
		let newly = total.saturating_sub(self.awards_emitted);
		self.awards_emitted = total;
		newly
	}

	pub fn accumulated(self) -> Duration {
		Duration::from_micros(self.accumulated_us.min(u64::MAX as u128) as u64)
	}
}

/// Timer identity. `tag` is set only when [`crate::OverlapPolicy::Stack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerKey {
	pub rule: RuleId,
	pub cell: OccupancyMatchKey,
	pub tag: Option<ConstructionTag>,
}

impl TimerKey {
	pub fn new(rule: RuleId, cell: OccupancyMatchKey, tag: Option<ConstructionTag>) -> Self {
		Self { rule, cell, tag }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn exact_interval_boundary_awards_once() -> anyhow::Result<()> {
		let mut timer = OccupancyTimer::default();
		let interval = Duration::from_secs(5);
		assert_eq!(timer.tick(Duration::from_secs(5), interval), 1);
		assert_eq!(timer.tick(Duration::from_secs(0), interval), 0);
		Ok(())
	}

	#[test]
	fn sub_interval_does_not_award() -> anyhow::Result<()> {
		let mut timer = OccupancyTimer::default();
		let interval = Duration::from_millis(1000);
		assert_eq!(timer.tick(Duration::from_millis(999), interval), 0);
		assert_eq!(timer.tick(Duration::from_millis(1), interval), 1);
		Ok(())
	}

	#[test]
	fn large_step_awards_each_elapsed_interval() -> anyhow::Result<()> {
		let mut timer = OccupancyTimer::default();
		let interval = Duration::from_secs(2);
		assert_eq!(timer.tick(Duration::from_millis(5500), interval), 2);
		assert_eq!(timer.tick(Duration::from_millis(500), interval), 1);
		Ok(())
	}
}
