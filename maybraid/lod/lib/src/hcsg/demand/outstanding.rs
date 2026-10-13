//! First-load and gate progress: outstanding work counts.

use super::super::bounds::HcsgClass;
use super::super::storage::Busy;
use super::HcsgDemand;

/// Outstanding work in a set of [`HcsgClass`]es.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outstanding {
	/// Live subscriptions that have not finished their first discovery.
	pub undiscovered: u64,
	/// Discovered ids not yet generated, summed across those subscriptions.
	pub remaining: u64,
}

impl HcsgDemand {
	/// Outstanding work in `classes`. A subscription is undiscovered until
	/// its first discovery finishes. Finished subscriptions (including a
	/// panic during first discovery) do not count. Replacement and
	/// unsubscribe drop their counts with the old id.
	pub fn try_outstanding(&self, classes: &[HcsgClass]) -> Result<Outstanding, Busy> {
		let state = self.try_lock()?;
		let mut outstanding = Outstanding::default();
		for subscription in state.subscriptions.values() {
			if subscription.done || !classes.contains(&subscription.class) {
				continue;
			}
			match subscription.discovered_len {
				None => outstanding.undiscovered += 1,
				Some(n) => {
					outstanding.remaining += n.saturating_sub(subscription.cursor) as u64;
				}
			}
		}
		Ok(outstanding)
	}
}
