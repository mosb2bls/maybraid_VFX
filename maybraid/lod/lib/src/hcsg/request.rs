//! Shared request/subscription handle for generation and presentation.
//!
//! Owns the channel's wanted regions and the live demand subscription. Adapters
//! keep their own work: [`super::generation::Generated`] never reads results;
//! [`super::presentation::Presented`] keeps hosts, cursor, and retirement.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

use super::bounds::{HcsgClass, HcsgRegions};
use super::context::GenerationScheme;
use super::demand::{HcsgDemand, SubscriptionId};
use super::storage::Busy;

/// Demand-facing request identity and subscription liveness for one scheme.
#[derive(Debug)]
pub(crate) struct HcsgRequest {
	wanted: Vec<Aabb3d>,
	focus: Option<Vec3>,
	class: HcsgClass,
	subscription: Option<SubscriptionId>,
	requested: Vec<Aabb3d>,
}

impl Default for HcsgRequest {
	fn default() -> Self {
		Self {
			wanted: Vec::new(),
			focus: None,
			class: HcsgClass::Near,
			subscription: None,
			requested: Vec::new(),
		}
	}
}

/// Outcome of [`HcsgRequest::sync`] when the subscription actually changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestSync {
	/// The previous id existed but was no longer live (epoch drop or cancel).
	pub epoch_advanced: bool,
}

impl HcsgRequest {
	/// Stamp the latest channel boxes, focus, and class. Change detection still
	/// keys off boxes only, matching [`HcsgRegions`] producers.
	pub(crate) fn apply_regions<C>(&mut self, latest: &HcsgRegions<C>) {
		self.wanted = latest.boxes.clone();
		self.focus = latest.focus;
		self.class = latest.class;
	}

	pub(crate) fn wanted(&self) -> &[Aabb3d] {
		&self.wanted
	}

	pub(crate) fn focus(&self) -> Option<Vec3> {
		self.focus
	}

	pub(crate) fn subscription(&self) -> Option<SubscriptionId> {
		self.subscription
	}

	/// Forget a subscription the adapter learned is gone (`try_read` → `None`).
	pub(crate) fn clear_subscription(&mut self) {
		self.subscription = None;
	}

	/// Align the subscription with current wanted regions.
	///
	/// `Ok(None)`: already aligned. `Ok(Some)`: replaced or dropped.
	/// `Err(Busy)`: demand lock contended; caller defers.
	pub(crate) fn sync<T: GenerationScheme>(
		&mut self,
		demand: &HcsgDemand,
	) -> Result<Option<RequestSync>, Busy> {
		let live = match self.subscription {
			Some(subscription) => demand.try_is_live(subscription)?,
			None => false,
		};
		let wants = !self.wanted.is_empty();
		if self.wanted == self.requested && live == wants {
			return Ok(None);
		}
		let epoch_advanced = self.subscription.is_some() && !live;
		if epoch_advanced {
			self.subscription = None;
		}
		self.requested = self.wanted.clone();
		if wants {
			let regions = self.wanted.clone();
			self.subscription =
				Some(demand.subscribe::<T>(self.subscription, regions, self.focus, self.class));
		} else if let Some(subscription) = self.subscription.take() {
			demand.unsubscribe(subscription);
		}
		Ok(Some(RequestSync { epoch_advanced }))
	}
}

#[cfg(test)]
mod tests {
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;

	use crate::gen::{Id, OriginalId};
	use crate::hcsg::demand::SubscriptionId;
	use crate::hcsg::{GenerationContext, GenerationScheme, HcsgDemand};

	use super::{HcsgClass, HcsgRegions, HcsgRequest, RequestSync};

	fn sync(request: &mut HcsgRequest, demand: &HcsgDemand) -> anyhow::Result<Option<RequestSync>> {
		request.sync::<Probe>(demand).map_err(|_| anyhow::anyhow!("demand busy"))
	}

	fn live(demand: &HcsgDemand, id: SubscriptionId) -> anyhow::Result<bool> {
		demand.try_is_live(id).map_err(|_| anyhow::anyhow!("demand busy"))
	}

	struct Probe;

	impl GenerationScheme for Probe {
		fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
			Vec::new()
		}

		fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
			None
		}
	}

	fn span(x: f32, width: f32) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
	}

	fn regions(boxes: Vec<Aabb3d>) -> HcsgRegions<()> {
		HcsgRegions::new(boxes, None, HcsgClass::Near)
	}

	#[test]
	fn sync_replaces_the_subscription_when_bounds_change() -> anyhow::Result<()> {
		let demand = HcsgDemand::default();
		let mut request = HcsgRequest::default();
		request.apply_regions(&regions(vec![span(0.0, 1.0)]));
		let first = sync(&mut request, &demand)?.ok_or_else(|| {
			anyhow::anyhow!("empty request should subscribe when wanted is non-empty")
		})?;
		anyhow::ensure!(!first.epoch_advanced, "first subscribe is not an epoch drop");
		let first_id =
			request.subscription().ok_or_else(|| anyhow::anyhow!("first subscription"))?;
		anyhow::ensure!(live(&demand, first_id)?, "first subscription should be live");

		request.apply_regions(&regions(vec![span(2.0, 1.0)]));
		let replaced = sync(&mut request, &demand)?
			.ok_or_else(|| anyhow::anyhow!("changed bounds should replace"))?;
		anyhow::ensure!(!replaced.epoch_advanced, "a live replace is not an epoch drop");
		let second_id = request
			.subscription()
			.ok_or_else(|| anyhow::anyhow!("replacement subscription"))?;
		anyhow::ensure!(first_id != second_id, "changed bounds mint a new subscription id");
		anyhow::ensure!(!live(&demand, first_id)?, "replaced predecessor is gone");
		anyhow::ensure!(live(&demand, second_id)?, "replacement should be live");
		Ok(())
	}

	#[test]
	fn sync_resubscribes_when_the_subscription_is_dead() -> anyhow::Result<()> {
		let demand = HcsgDemand::default();
		let mut request = HcsgRequest::default();
		request.apply_regions(&regions(vec![span(0.0, 1.0)]));
		sync(&mut request, &demand)?;
		let dead_id = request.subscription().ok_or_else(|| anyhow::anyhow!("live id"))?;
		demand.advance_epoch();
		anyhow::ensure!(!live(&demand, dead_id)?, "epoch drop kills the id");

		let transition = sync(&mut request, &demand)?
			.ok_or_else(|| anyhow::anyhow!("dead subscription should resubscribe"))?;
		anyhow::ensure!(transition.epoch_advanced, "a missing live id is an epoch drop");
		let live_id = request.subscription().ok_or_else(|| anyhow::anyhow!("resubscribed id"))?;
		anyhow::ensure!(live_id != dead_id, "epoch drop must not reuse the dead id");
		anyhow::ensure!(live(&demand, live_id)?, "resubscribe should be live");
		Ok(())
	}

	#[test]
	fn sync_is_a_noop_when_wanted_and_liveness_already_match() -> anyhow::Result<()> {
		let demand = HcsgDemand::default();
		let mut request = HcsgRequest::default();
		request.apply_regions(&regions(vec![span(0.0, 1.0)]));
		sync(&mut request, &demand)?;
		let id = request.subscription().ok_or_else(|| anyhow::anyhow!("subscribed"))?;
		anyhow::ensure!(sync(&mut request, &demand)?.is_none(), "aligned request stays put");
		anyhow::ensure!(request.subscription() == Some(id));
		Ok(())
	}
}
