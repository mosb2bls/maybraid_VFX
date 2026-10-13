use bevy::prelude::*;

use crate::band::RoutingSettings;
use crate::plan::{FailedEdge, RoutePlan};
use crate::probe::RouteProbe;

/// Long-range corridor memory. Writes a nearby [`movement_intelligence::MovementObjective::Reach`]
/// through [`crate::write_route_objectives`]; this component does not drive the capsule.
#[derive(Component, Clone, Debug)]
pub struct RoutingIntelligenceUser {
	pub settings: RoutingSettings,
	pub destination: Option<Vec3>,
	pub plan: RoutePlan,
	pub hop: usize,
	pub failed: Vec<FailedEdge>,
	pub dirty: bool,
}

impl RoutingIntelligenceUser {
	pub fn new(settings: RoutingSettings) -> Self {
		Self { settings, ..Self::default() }
	}

	pub fn set_destination(&mut self, destination: Vec3) {
		if self.destination.is_some_and(|current| {
			Vec2::new(current.x, current.z).distance(Vec2::new(destination.x, destination.z)) < 0.5
				&& (current.y - destination.y).abs() < 0.5
		}) {
			return;
		}
		self.destination = Some(destination);
		self.dirty = true;
		self.hop = 0;
		self.failed.clear();
	}

	pub fn clear_destination(&mut self) {
		self.destination = None;
		self.plan = RoutePlan::default();
		self.hop = 0;
		self.dirty = false;
	}

	pub fn replan(&mut self, from: Vec3, probe: &impl RouteProbe) {
		let Some(goal) = self.destination else {
			self.plan = RoutePlan::default();
			self.hop = 0;
			self.dirty = false;
			return;
		};
		let previous = self.plan.clone();
		self.plan =
			RoutePlan::plan(from, goal, &self.settings, probe, Some(&previous), &self.failed);
		self.hop = 0;
		self.dirty = false;
	}

	pub fn needs_plan(&self, from: Vec3) -> bool {
		if self.destination.is_none() {
			return false;
		}
		self.dirty || self.plan.finest_waypoints().len() < 2 || self.off_corridor(from)
	}

	pub fn current_hop(&self, from: Vec3) -> Option<Vec3> {
		let points = self.plan.finest_waypoints();
		if points.len() < 2 {
			return self.destination;
		}
		let hop = self.hop.saturating_add(1).min(points.len() - 1);
		let point = points[hop];
		if Vec2::new(point.x - from.x, point.z - from.z).length() <= self.settings.arrival_radius {
			return points.get(hop + 1).copied().or(self.destination);
		}
		Some(point)
	}

	pub fn advance(&mut self, from: Vec3) {
		let points = self.plan.finest_waypoints();
		if points.len() < 2 {
			return;
		}
		let next = self.hop.saturating_add(1).min(points.len() - 1);
		let point = points[next];
		if Vec2::new(point.x - from.x, point.z - from.z).length() <= self.settings.arrival_radius {
			self.hop = next;
		}
	}

	fn off_corridor(&self, from: Vec3) -> bool {
		let Some(hop) = self.current_hop(from) else {
			return false;
		};
		let segment = self.plan.finest().map(|layer| layer.segment).unwrap_or(32.0);
		Vec2::new(hop.x - from.x, hop.z - from.z).length() > segment * 2.5
	}
}

impl Default for RoutingIntelligenceUser {
	fn default() -> Self {
		Self {
			settings: RoutingSettings::default(),
			destination: None,
			plan: RoutePlan::default(),
			hop: 0,
			failed: Vec::new(),
			dirty: false,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::probe::RouteProbe;

	struct Flat;

	impl RouteProbe for Flat {
		fn ground(&self, xz: Vec2, _hint_y: f32) -> Option<Vec3> {
			Some(Vec3::new(xz.x, 0.0, xz.y))
		}

		fn blocked(&self, _from_hip: Vec3, _to_hip: Vec3) -> bool {
			false
		}
	}

	#[test]
	fn hop_advances_when_the_agent_arrives() -> anyhow::Result<()> {
		let mut user = RoutingIntelligenceUser::new(RoutingSettings::from_segments([40.0]));
		user.set_destination(Vec3::X * 80.0);
		user.replan(Vec3::ZERO, &Flat);
		assert!(user.plan.finest_waypoints().len() >= 2);
		let first = user.current_hop(Vec3::ZERO).ok_or_else(|| anyhow::anyhow!("hop"))?;
		user.advance(first);
		let second = user.current_hop(first);
		assert!(second.is_some_and(|point| (point - first).length() > 1.0));
		Ok(())
	}
}
