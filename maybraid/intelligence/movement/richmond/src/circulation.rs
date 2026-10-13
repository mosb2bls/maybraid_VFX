//! World-space storey slabs and stairwell links, derived from Richmond IR.

use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use buildings::{ConnectingStairwell, MixedUseLesHallesStorey};

/// One Les Halles storey volume in world space.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CirculationStorey {
	pub id: u32,
	pub bounds: Aabb3d,
	pub floor_y: f32,
}

impl CirculationStorey {
	pub fn contains_xz(&self, p: Vec3) -> bool {
		let min = Vec3::from(self.bounds.min);
		let max = Vec3::from(self.bounds.max);
		p.x >= min.x - 0.35 && p.x <= max.x + 0.35 && p.z >= min.z - 0.35 && p.z <= max.z + 0.35
	}

	pub fn contains_y(&self, p: Vec3) -> bool {
		let min = Vec3::from(self.bounds.min);
		let max = Vec3::from(self.bounds.max);
		p.y >= min.y - 0.5 && p.y <= max.y + 0.5
	}

	pub fn contains(&self, p: Vec3) -> bool {
		self.contains_xz(p) && self.contains_y(p)
	}
}

/// One climb between storeys: mouth, tread polyline (walk surface), landing.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CirculationStairwell {
	pub from_storey: u32,
	pub to_storey: u32,
	pub well: Aabb3d,
	pub mouth: Vec3,
	pub landing: Vec3,
	/// Walk-surface points, lower storey toward upper.
	pub polyline: Vec<Vec3>,
}

impl CirculationStairwell {
	pub fn contains_actor(&self, p: Vec3) -> bool {
		let min = Vec3::from(self.well.min);
		let max = Vec3::from(self.well.max);
		p.x >= min.x - 0.6
			&& p.x <= max.x + 0.6
			&& p.z >= min.z - 0.6
			&& p.z <= max.z + 0.6
			&& p.y >= min.y - 1.0
			&& p.y <= max.y + 1.2
	}

	/// Remaining walk-surface points at or above `from` (ascending).
	pub fn remaining_ascending(&self, from: Vec3) -> Vec<Vec3> {
		if self.polyline.is_empty() {
			return vec![self.landing];
		}
		let start = nearest_point_index(&self.polyline, from);
		self.polyline[start..].to_vec()
	}

	pub fn oriented_polyline(&self, going_up: bool, from: Vec3) -> Vec<Vec3> {
		if going_up {
			self.remaining_ascending(from)
		} else {
			let mut pts = self.polyline.clone();
			pts.reverse();
			if pts.is_empty() {
				return vec![self.mouth];
			}
			let start = nearest_point_index(&pts, from);
			pts[start..].to_vec()
		}
	}

	/// Polyline points from `from` toward a destination on this stair, stopping
	/// approximately `standoff` metres of stair travel before it.
	pub fn route_toward(&self, from: Vec3, destination: Vec3, standoff: f32) -> Vec<Vec3> {
		if self.polyline.is_empty() {
			return Vec::new();
		}
		let start = nearest_point_index(&self.polyline, from);
		let goal = nearest_point_index(&self.polyline, destination);
		let mut stop = goal;
		let mut remaining = standoff.max(0.0);
		while stop != start && remaining > 0.0 {
			let next = if stop > start { stop - 1 } else { stop + 1 };
			remaining -= self.polyline[stop].distance(self.polyline[next]);
			stop = next;
		}
		if start <= stop {
			self.polyline[start..=stop].to_vec()
		} else {
			self.polyline[stop..=start].iter().rev().copied().collect()
		}
	}
}

fn nearest_point_index(points: &[Vec3], from: Vec3) -> usize {
	points
		.iter()
		.enumerate()
		.min_by(|(_, a), (_, b)| a.distance_squared(from).total_cmp(&b.distance_squared(from)))
		.map(|(i, _)| i)
		.unwrap_or(0)
}

/// Stamp a storey host from a mixed-use Les Halles floor.
pub fn circulation_from_storey(
	id: u32,
	storey: &MixedUseLesHallesStorey,
	world: Transform,
) -> CirculationStorey {
	let plan = storey.floor_plan();
	let hx = plan.outer.x * 0.5;
	let hz = plan.outer.y * 0.5;
	let c = plan.center_xz;
	let h = plan.storey_height.max(1e-3);
	let local = Aabb3d::from_min_max(
		Vec3::new(c.x - hx, c.y, c.z - hz),
		Vec3::new(c.x + hx, c.y + h, c.z + hz),
	);
	CirculationStorey {
		id,
		bounds: transform_aabb(local, world),
		floor_y: world.transform_point(Vec3::new(c.x, c.y, c.z)).y,
	}
}

/// Stamp a stairwell link. `from_storey` is the lower floor index.
pub fn circulation_from_stairwell(
	from_storey: u32,
	to_storey: u32,
	stairwell: &ConnectingStairwell,
	world: Transform,
) -> CirculationStairwell {
	let well = stairwell.well();
	let outward = xz_to_vec3(well.walk_on.into_xz());
	let mouth_local = well.side_mid(well.walk_on, well.bottom_y()) + outward * 1.35;
	let mut polyline = vec![mouth_local];
	let mut tread_points = Vec::new();
	let mut lineup = None;
	for node in stairwell.stairs() {
		for (center, rotation, size) in node.tread_cuboids() {
			let top = center + rotation * Vec3::Y * (size.y * 0.5);
			if lineup.is_none() {
				let travel = (rotation * Vec3::X).with_y(0.0).normalize_or_zero();
				let trailing = top - travel * (size.x * 0.5);
				let mut entry = trailing - travel * 0.45;
				entry.y = well.bottom_y();
				lineup = Some(entry);
			}
			tread_points.push(top);
		}
	}
	if let Some(lineup) = lineup {
		// Use the bounded Manhattan corner between the mouth and lineup. Projecting
		// the other way makes the route reverse onto the stairs; adding a run-in
		// behind the lineup can push it outside the available floor and into a wall.
		let mouth = polyline[0];
		let travel = tread_points
			.first()
			.map(|first| (*first - lineup).with_y(0.0).normalize_or_zero())
			.unwrap_or(Vec3::ZERO);
		let corner = mouth + travel * (lineup - mouth).dot(travel);
		if (corner - mouth).with_y(0.0).length() > 0.08 {
			polyline.push(corner);
		}
		polyline.push(lineup);
	}
	polyline.extend(tread_points);
	let landing_local = stairwell
		.last_tread_end()
		.map(|end| {
			let leading = end.leading_mid();
			let travel = end.travel.normalize_or_zero();
			let outward = well.walk_off.into_xz();
			let face = well.side_mid(well.walk_off, well.top_y()).xz();
			let toward_face = travel.dot(outward);
			let exit = if toward_face > 0.1 {
				let to_face = ((face - leading).dot(outward) / toward_face).max(0.0);
				leading + travel * (to_face + 0.65)
			} else {
				// Spiral/tangent fallback: leave at the tread's lateral position,
				// never pull toward the conceptual center of the walk-off face.
				leading + outward * ((face - leading).dot(outward).max(0.0) + 0.65)
			};
			Vec3::new(exit.x, well.top_y(), exit.y)
		})
		.unwrap_or_else(|| {
			let off = xz_to_vec3(well.walk_off.into_xz());
			well.side_mid(well.walk_off, well.top_y()) + off * 0.65
		});
	polyline.push(landing_local);
	CirculationStairwell {
		from_storey,
		to_storey,
		well: transform_aabb(well.bounds, world),
		mouth: world.transform_point(mouth_local),
		landing: world.transform_point(landing_local),
		polyline: polyline.into_iter().map(|p| world.transform_point(p)).collect(),
	}
}

fn xz_to_vec3(v: bevy_math::Vec2) -> Vec3 {
	Vec3::new(v.x, 0.0, v.y)
}

fn transform_aabb(bounds: Aabb3d, world: Transform) -> Aabb3d {
	let corners = [
		Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
		Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
		Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
		Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
		Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
		Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
		Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
		Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
	];
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for c in corners {
		let w = world.transform_point(c);
		min = min.min(w);
		max = max.max(w);
	}
	Aabb3d::from_min_max(min, max)
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::panels::PanelStyle;
	use buildings::{StairwellKind, WellAabb, WellSide};

	#[test]
	fn rectangular_well_polyline_rises() -> anyhow::Result<()> {
		let well = WellAabb::from_plan(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(3.5, 3.6, 3.5),
			WellSide::NegZ,
			WellSide::NegZ,
			0.4,
		);
		let stairwell = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			well,
			StairwellKind::Rectangular,
		);
		let link = circulation_from_stairwell(0, 1, &stairwell, Transform::IDENTITY);
		assert!(link.polyline.len() >= 3, "{}", link.polyline.len());
		let first = link.polyline.first().copied().unwrap_or_default();
		let last = link.polyline.last().copied().unwrap_or_default();
		assert!(last.y > first.y + 1.0, "{} vs {}", last.y, first.y);
		assert!(link.landing.y > link.mouth.y);
		let corner = link.polyline[1];
		let lineup = link.polyline[2];
		let first_tread = link.polyline[3];
		let first_node = stairwell.stairs().first().expect("first flight");
		let travel = (first_node.placement.rotation() * Vec3::X).with_y(0.0).normalize();
		let outer_run = (corner - link.mouth).with_y(0.0).normalize();
		let cross_run = (lineup - corner).with_y(0.0).normalize();
		assert!(
			outer_run.dot(cross_run).abs() < 1e-4,
			"stair lead-up should use a right-angle turn: {outer_run}, {cross_run}"
		);
		assert!(
			(corner - lineup).dot(travel).abs() < 1e-4,
			"corner must not extend beyond the lineup along stair travel: {corner} vs {lineup}"
		);
		assert!(
			corner.distance(link.mouth) <= lineup.distance(link.mouth),
			"corner should remain inside the mouth-lineup bounds: {corner}"
		);
		let entry_direction = (first_tread - lineup).with_y(0.0).normalize();
		assert!(
			entry_direction.dot(travel) > 0.99,
			"entry should align with first flight: {entry_direction} vs {travel}"
		);
		assert!((lineup.y - well.bottom_y()).abs() < 1e-4);
		let last_tread = link.polyline[link.polyline.len() - 2];
		let exit = *link.polyline.last().expect("stair exit");
		let last_node = stairwell.stairs().last().expect("last flight");
		let exit_travel = (last_node.placement.rotation() * Vec3::X).with_y(0.0).normalize();
		let exit_direction = (exit - last_tread).with_y(0.0).normalize();
		assert!(
			exit_direction.dot(exit_travel) > 0.99,
			"exit should continue straight from last flight: {exit_direction} vs {exit_travel}"
		);
		let outward = well.walk_off.into_xz();
		let face = well.side_mid(well.walk_off, well.top_y()).xz();
		assert!(
			(exit.xz() - face).dot(outward) > 0.6,
			"exit should finish outside the well on the next floor: exit={exit}, face={face}"
		);
		Ok(())
	}

	#[test]
	fn route_toward_stops_on_the_stair_before_the_destination() -> anyhow::Result<()> {
		let link = CirculationStairwell {
			from_storey: 0,
			to_storey: 1,
			well: Aabb3d::from_min_max(Vec3::ZERO, Vec3::splat(4.0)),
			mouth: Vec3::ZERO,
			landing: Vec3::new(4.0, 4.0, 0.0),
			polyline: (0..=4).map(|i| Vec3::new(i as f32, i as f32, 0.0)).collect(),
		};
		let route = link.route_toward(link.mouth, link.polyline[4], 1.5);
		assert_eq!(route.first().copied(), Some(link.polyline[0]));
		assert_eq!(route.last().copied(), Some(link.polyline[2]));
		assert!(!route.contains(&link.landing));
		let descending = link.route_toward(link.landing, link.mouth, 1.5);
		assert_eq!(descending.first().copied(), Some(link.polyline[4]));
		assert_eq!(descending.last().copied(), Some(link.polyline[2]));
		assert!(!descending.contains(&link.mouth));
		Ok(())
	}

	#[test]
	fn storey_contains_center() -> anyhow::Result<()> {
		let bounds = Aabb3d::from_min_max(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 3.5, 4.0));
		let storey = CirculationStorey { id: 0, bounds, floor_y: 0.0 };
		assert!(storey.contains(Vec3::new(0.0, 1.0, 0.0)));
		assert!(!storey.contains(Vec3::new(0.0, 8.0, 0.0)));
		Ok(())
	}
}
