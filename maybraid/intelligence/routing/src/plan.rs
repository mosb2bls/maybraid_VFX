use bevy::prelude::*;

use crate::band::RoutingSettings;
use crate::probe::RouteProbe;

const MAX_HOPS_PER_LAYER: usize = 64;

/// Committed waypoints at one band's segment length.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerPlan {
	pub segment: f32,
	pub waypoints: Vec<Vec3>,
}

/// Hierarchical corridor. Layer 0 is coarsest.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoutePlan {
	pub layers: Vec<LayerPlan>,
}

impl RoutePlan {
	pub fn finest(&self) -> Option<&LayerPlan> {
		self.layers.last()
	}

	pub fn finest_waypoints(&self) -> &[Vec3] {
		self.finest().map(|layer| layer.waypoints.as_slice()).unwrap_or(&[])
	}

	/// Build coarse-to-fine corridors from `from` toward `goal`.
	pub fn plan(
		from: Vec3,
		goal: Vec3,
		settings: &RoutingSettings,
		probe: &impl RouteProbe,
		previous: Option<&RoutePlan>,
		failed: &[FailedEdge],
	) -> Self {
		let sampler = RouteSampler { settings, probe };
		let hint_y = from.y;
		let Some(start) = sampler.snap_origin(from, hint_y) else {
			return RoutePlan::default();
		};
		let end = sampler.snap_origin(goal, goal.y.max(hint_y)).unwrap_or(goal);
		if settings.bands.is_empty() {
			return RoutePlan {
				layers: vec![LayerPlan {
					segment: RouteGeom::xz(start, end).max(1.0),
					waypoints: vec![start, end],
				}],
			};
		}

		let mut parent = vec![start, end];
		let mut layers = Vec::with_capacity(settings.bands.len());
		for (index, band) in settings.bands.iter().enumerate() {
			let previous_layer = previous.and_then(|plan| plan.layers.get(index));
			let waypoints = LayerPlan::refine(
				start,
				end,
				&parent,
				*band,
				index as u8,
				settings,
				probe,
				previous_layer,
				failed,
			);
			parent = waypoints.clone();
			layers.push(LayerPlan { segment: band.segment, waypoints });
		}
		RoutePlan { layers }
	}
}

impl LayerPlan {
	fn refine(
		start: Vec3,
		goal: Vec3,
		parent: &[Vec3],
		band: crate::band::RoutingBand,
		layer: u8,
		settings: &RoutingSettings,
		probe: &impl RouteProbe,
		previous: Option<&LayerPlan>,
		failed: &[FailedEdge],
	) -> Vec<Vec3> {
		let sampler = RouteSampler { settings, probe };
		let evaluator = ChordEvaluator { layer, settings, probe, previous, failed };
		let slack = band.lateral_span.max(band.segment * 0.25);
		let mut waypoints = vec![start];
		let mut current = start;
		for _ in 0..MAX_HOPS_PER_LAYER {
			if RouteGeom::xz(current, goal) <= band.segment * 0.55 {
				break;
			}
			let (forward, perp) = RouteGeom::corridor_frame(parent, current, goal);
			let step = band.segment.min(RouteGeom::xz(current, goal));
			let mut scores = Vec::new();
			for sample in RouteGeom::candidate_offsets(current, forward, perp, step, band) {
				if RouteGeom::polyline_distance(parent, sample) > slack + 1e-3 {
					continue;
				}
				let Some(point) = sampler.snap_origin(sample, current.y) else {
					continue;
				};
				scores.push(evaluator.score(current, point));
			}
			let best = ChordScore::pick_best(&scores);
			let Some(next) = best else {
				break;
			};
			if RouteGeom::xz(next.point, current) < 0.25 {
				break;
			}
			waypoints.push(next.point);
			current = next.point;
		}
		if waypoints
			.last()
			.is_none_or(|point| RouteGeom::xz(*point, goal) > settings.arrival_radius)
		{
			waypoints.push(goal);
		}
		waypoints
	}
}

/// A chord that a finer layer could not walk; coarse replans add cost here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FailedEdge {
	pub layer: u8,
	pub a: Vec2,
	pub b: Vec2,
}

impl FailedEdge {
	pub fn new(layer: u8, a: Vec3, b: Vec3) -> Self {
		Self { layer, a: Vec2::new(a.x, a.z), b: Vec2::new(b.x, b.z) }
	}

	pub fn overlaps(self, a: Vec3, b: Vec3, slop: f32) -> bool {
		let p = Vec2::new(a.x, a.z);
		let q = Vec2::new(b.x, b.z);
		RouteGeom::segment_distance(self.a, self.b, p) <= slop
			&& RouteGeom::segment_distance(self.a, self.b, q) <= slop
			|| RouteGeom::segment_distance(p, q, self.a) <= slop
				&& RouteGeom::segment_distance(p, q, self.b) <= slop
	}
}

#[derive(Clone, Copy, Debug)]
struct ChordScore {
	point: Vec3,
	cost: f32,
	blocked: bool,
	cliff: bool,
}

impl ChordScore {
	fn pick_best(scores: &[Self]) -> Option<Self> {
		let legal = scores.iter().copied().filter(|score| !score.blocked && !score.cliff);
		legal
			.min_by(|a, b| a.cost.total_cmp(&b.cost))
			.or_else(|| scores.iter().copied().min_by(|a, b| a.cost.total_cmp(&b.cost)))
	}
}

struct RouteSampler<'a, P: RouteProbe> {
	settings: &'a RoutingSettings,
	probe: &'a P,
}

impl<P: RouteProbe> RouteSampler<'_, P> {
	fn snap_origin(&self, point: Vec3, hint_y: f32) -> Option<Vec3> {
		let ground = self.probe.ground(Vec2::new(point.x, point.z), hint_y)?;
		Some(Vec3::new(ground.x, ground.y + self.settings.feet_below_origin, ground.z))
	}

	fn hip(&self, origin: Vec3) -> Vec3 {
		Vec3::new(
			origin.x,
			origin.y - self.settings.feet_below_origin + self.settings.hip_height,
			origin.z,
		)
	}
}

struct ChordEvaluator<'a, P: RouteProbe> {
	layer: u8,
	settings: &'a RoutingSettings,
	probe: &'a P,
	previous: Option<&'a LayerPlan>,
	failed: &'a [FailedEdge],
}

impl<P: RouteProbe> ChordEvaluator<'_, P> {
	fn score(&self, from: Vec3, to: Vec3) -> ChordScore {
		let sampler = RouteSampler { settings: self.settings, probe: self.probe };
		let length = RouteGeom::xz(from, to).max(0.01);
		let hip_from = sampler.hip(from);
		let hip_to = sampler.hip(to);
		let blocked = self.probe.blocked(hip_from, hip_to);
		let (cliff, max_drop) = self.chord_drop(from, to);
		let mut cost = length + self.settings.weight_drop * max_drop;
		if blocked {
			cost += self.settings.blocked_cost;
		}
		if cliff {
			cost += self.settings.cliff_cost;
		}
		if self
			.failed
			.iter()
			.any(|edge| edge.layer == self.layer && edge.overlaps(from, to, 4.0))
		{
			cost += self.settings.failed_cost;
		}
		if let Some(previous) = self.previous {
			let pull = RouteGeom::polyline_distance(&previous.waypoints, to);
			cost += pull * self.settings.continuity;
		}
		ChordScore { point: to, cost, blocked, cliff }
	}

	fn chord_drop(&self, from: Vec3, to: Vec3) -> (bool, f32) {
		let step = self
			.settings
			.bands
			.iter()
			.find(|band| band.segment + 1e-3 >= RouteGeom::xz(from, to))
			.map(|band| band.probe_step)
			.unwrap_or_else(|| (RouteGeom::xz(from, to) / 4.0).max(2.0));
		let last_y = from.y - self.settings.feet_below_origin;
		let mut max_drop = 0.0_f32;
		for point in RouteGeom::chord_samples(from, to, step) {
			let Some(ground) = self.probe.ground(Vec2::new(point.x, point.z), point.y) else {
				return (true, self.settings.max_fall + 1.0);
			};
			let drop = (last_y - ground.y).max(0.0);
			if drop > self.settings.max_fall + 0.04 {
				return (true, drop);
			}
			max_drop = max_drop.max(drop);
		}
		(false, max_drop)
	}
}

struct RouteGeom;

impl RouteGeom {
	fn xz(a: Vec3, b: Vec3) -> f32 {
		Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z))
	}

	fn candidate_offsets(
		current: Vec3,
		forward: Vec3,
		perp: Vec3,
		step: f32,
		band: crate::band::RoutingBand,
	) -> Vec<Vec3> {
		let mut points = vec![current + forward * step];
		let sides = band.laterals;
		if sides == 0 || band.lateral_span <= 1e-4 {
			return points;
		}
		for i in 1..=sides {
			let offset = band.lateral_span * (i as f32 / sides as f32);
			points.push(current + forward * step + perp * offset);
			points.push(current + forward * step - perp * offset);
		}
		points
	}

	fn corridor_frame(parent: &[Vec3], current: Vec3, goal: Vec3) -> (Vec3, Vec3) {
		let remaining = Vec3::new(goal.x - current.x, 0.0, goal.z - current.z);
		let to_goal =
			if remaining.length_squared() < 1e-6 { Vec3::X } else { remaining.normalize() };
		let along_parent = Self::parent_tangent(parent, current).unwrap_or(to_goal);
		let blended = along_parent * 0.35 + to_goal * 0.65;
		let forward = blended.try_normalize().unwrap_or(to_goal);
		let perp = Vec3::new(-forward.z, 0.0, forward.x);
		(forward, perp)
	}

	fn parent_tangent(parent: &[Vec3], current: Vec3) -> Option<Vec3> {
		if parent.len() < 2 {
			return None;
		}
		let mut best = (f32::MAX, Vec3::X);
		for window in parent.windows(2) {
			let a = window[0];
			let b = window[1];
			let delta = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
			let len = delta.length();
			if len < 1e-4 {
				continue;
			}
			let t = ((Vec2::new(current.x - a.x, current.z - a.z))
				.dot(Vec2::new(delta.x, delta.z))
				/ (len * len))
				.clamp(0.0, 1.0);
			let closest = a + delta * t;
			let dist = Self::xz(current, closest);
			if dist < best.0 {
				best = (dist, delta / len);
			}
		}
		(best.0 < f32::MAX).then_some(best.1)
	}

	fn chord_samples(from: Vec3, to: Vec3, step: f32) -> Vec<Vec3> {
		let delta = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
		let len = delta.length();
		if len < 1e-4 {
			return vec![to];
		}
		let n = ((len / step.max(0.5)).ceil() as usize).max(1);
		(1..=n)
			.map(|i| {
				let t = i as f32 / n as f32;
				Vec3::new(from.x + delta.x * t, from.y.lerp(to.y, t), from.z + delta.z * t)
			})
			.collect()
	}

	fn polyline_distance(points: &[Vec3], sample: Vec3) -> f32 {
		if points.is_empty() {
			return 0.0;
		}
		if points.len() == 1 {
			return Self::xz(points[0], sample);
		}
		points
			.windows(2)
			.map(|window| {
				Self::segment_distance(
					Vec2::new(window[0].x, window[0].z),
					Vec2::new(window[1].x, window[1].z),
					Vec2::new(sample.x, sample.z),
				)
			})
			.fold(f32::MAX, f32::min)
	}

	fn segment_distance(a: Vec2, b: Vec2, p: Vec2) -> f32 {
		let ab = b - a;
		let len_sq = ab.length_squared();
		if len_sq < 1e-8 {
			return (p - a).length();
		}
		let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
		(a + ab * t - p).length()
	}
}

/// Build coarse-to-fine corridors from `from` toward `goal`.
pub fn plan_route(
	from: Vec3,
	goal: Vec3,
	settings: &RoutingSettings,
	probe: &impl RouteProbe,
	previous: Option<&RoutePlan>,
	failed: &[FailedEdge],
) -> RoutePlan {
	RoutePlan::plan(from, goal, settings, probe, previous, failed)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::band::RoutingSettings;
	use crate::probe::RouteProbe;

	struct MapProbe {
		buildings: Vec<(Vec2, Vec2)>,
		voids: Vec<(Vec2, Vec2)>,
		height: f32,
	}

	impl MapProbe {
		fn open(height: f32) -> Self {
			Self { buildings: Vec::new(), voids: Vec::new(), height }
		}

		fn in_box(min: Vec2, max: Vec2, p: Vec2) -> bool {
			p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
		}
	}

	impl RouteProbe for MapProbe {
		fn ground(&self, xz: Vec2, _hint_y: f32) -> Option<Vec3> {
			if self.voids.iter().any(|(min, max)| Self::in_box(*min, *max, xz)) {
				return None;
			}
			Some(Vec3::new(xz.x, self.height, xz.y))
		}

		fn blocked(&self, from_hip: Vec3, to_hip: Vec3) -> bool {
			let a = Vec2::new(from_hip.x, from_hip.z);
			let b = Vec2::new(to_hip.x, to_hip.z);
			self.buildings.iter().any(|(min, max)| segment_hits_aabb(a, b, *min, *max))
		}
	}

	fn segment_hits_aabb(start: Vec2, end: Vec2, min: Vec2, max: Vec2) -> bool {
		let d = end - start;
		let mut t0 = 0.0_f32;
		let mut t1 = 1.0_f32;
		for axis in 0..2 {
			let (p, delta, lo, hi) =
				if axis == 0 { (start.x, d.x, min.x, max.x) } else { (start.y, d.y, min.y, max.y) };
			if delta.abs() < 1e-8 {
				if p < lo || p > hi {
					return false;
				}
				continue;
			}
			let inv = 1.0 / delta;
			let mut u0 = (lo - p) * inv;
			let mut u1 = (hi - p) * inv;
			if u0 > u1 {
				core::mem::swap(&mut u0, &mut u1);
			}
			t0 = t0.max(u0);
			t1 = t1.min(u1);
			if t0 > t1 {
				return false;
			}
		}
		t1 >= 0.0 && t0 <= 1.0
	}

	#[test]
	fn open_ground_commits_band_spacing() -> anyhow::Result<()> {
		let settings = RoutingSettings::from_segments([80.0, 20.0]);
		let plan =
			plan_route(Vec3::ZERO, Vec3::X * 160.0, &settings, &MapProbe::open(0.0), None, &[]);
		assert_eq!(plan.layers.len(), 2);
		assert!((plan.layers[0].segment - 80.0).abs() < 1e-4);
		let coarse = &plan.layers[0].waypoints;
		assert!(coarse.len() >= 2);
		let first_hop = RouteGeom::xz(coarse[0], coarse[1]);
		assert!(
			(first_hop - 80.0).abs() < 12.0
				|| first_hop + 1.0 >= RouteGeom::xz(coarse[0], *coarse.last().unwrap()),
			"first hop {first_hop}"
		);
		Ok(())
	}

	#[test]
	fn building_forces_a_lateral_detour() -> anyhow::Result<()> {
		let settings = RoutingSettings::from_segments([40.0, 16.0]);
		let probe = MapProbe {
			buildings: vec![(Vec2::new(18.0, -8.0), Vec2::new(26.0, 8.0))],
			voids: Vec::new(),
			height: 0.0,
		};
		let plan = plan_route(Vec3::ZERO, Vec3::X * 80.0, &settings, &probe, None, &[]);
		let finest = plan.finest_waypoints();
		assert!(finest.len() >= 2);
		let max_abs_z = finest.iter().map(|point| point.z.abs()).fold(0.0_f32, f32::max);
		assert!(max_abs_z > 4.0, "expected a z detour, waypoints={finest:?}");
		Ok(())
	}

	#[test]
	fn void_is_treated_as_a_cliff() -> anyhow::Result<()> {
		let settings = RoutingSettings::from_segments([40.0]);
		let probe = MapProbe {
			buildings: Vec::new(),
			voids: vec![(Vec2::new(20.0, -12.0), Vec2::new(50.0, 12.0))],
			height: 0.0,
		};
		let plan = plan_route(Vec3::ZERO, Vec3::X * 90.0, &settings, &probe, None, &[]);
		let hops = plan.finest_waypoints();
		let crossed_void = hops.windows(2).any(|window| {
			window[0].x < 20.0
				&& window[1].x > 50.0
				&& window[0].z.abs() < 25.0
				&& window[1].z.abs() < 25.0
		});
		assert!(!crossed_void, "straight void hop in {hops:?}");
		Ok(())
	}

	#[test]
	fn failed_edge_makes_the_old_chord_more_expensive() -> anyhow::Result<()> {
		let settings = RoutingSettings::from_segments([40.0]);
		let probe = MapProbe::open(0.0);
		let failed = [FailedEdge::new(0, Vec3::ZERO, Vec3::X * 40.0)];
		let plan = plan_route(Vec3::ZERO, Vec3::X * 80.0, &settings, &probe, None, &failed);
		let first = plan.finest_waypoints().get(1).copied().unwrap_or(Vec3::X * 40.0);
		assert!(
			first.z.abs() > 1.0,
			"failed straight chord should prefer a lateral, got {first:?}"
		);
		Ok(())
	}
}
