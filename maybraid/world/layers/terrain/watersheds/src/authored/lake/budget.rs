//! Inscribed elliptical band budget for lake leaves.

use crate::authored::lake::LakeParams;
use crate::primitive::parameters::TARGET_RIM_WIDTH;
use bevy_math::Vec2;
use procedural_common::Bounds2;

/// Minimum water half-axis (world units); smaller budgets skip the stamp.
pub(crate) const MIN_WATER_RADIUS: f32 = 8.0;

/// Elliptical band budget derived from per-axis clearance at the lake centroid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LakeBandBudget {
	/// Water half-axes in the ellipse local frame.
	pub water_radii: Vec2,
	pub rotation: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	/// Plateau half-axes (`water + rim`) in the local frame.
	pub plateau_radii: Vec2,
	pub mu: f32,
}

impl LakeBandBudget {
	/// Characteristic (short) water half-axis.
	pub fn water_radius(&self) -> f32 {
		self.water_radii.min_element()
	}

	/// Characteristic (short) plateau half-axis.
	pub fn plateau_radius(&self) -> f32 {
		self.plateau_radii.min_element()
	}

	/// Budget water / rim / apron from inscribed clearance at `center`.
	///
	/// Rim + apron claim first (isotropic widths); water takes a noisy fraction
	/// of the leftover on each axis, blended between circular and full leaf
	/// aspect. A small rotation is applied and axes are rescaled so the rotated
	/// ellipse AABB still fits. `water_u01` / `rim_u01` / `aspect_u01` /
	/// `rotation_u11` should be stable per leaf.
	///
	/// Returns `None` when the leaf cannot host a meaningful three-band lake.
	pub fn try_inscribed(
		bounds: Bounds2,
		center: Vec2,
		params: LakeParams,
		water_u01: f32,
		rim_u01: f32,
		aspect_u01: f32,
		rotation_u11: f32,
	) -> Option<Self> {
		let min = bounds.min;
		let max = bounds.max;
		let room = Vec2::new(
			(center.x - min.x).min(max.x - center.x).max(0.0),
			(center.y - min.y).min(max.y - center.y).max(0.0),
		);
		let short_room = room.min_element();
		if short_room < MIN_WATER_RADIUS * 2.0 {
			return None;
		}
		let mu = params.mu.min(short_room * 0.2).max(0.0);
		let available = (room - Vec2::splat(mu)).max(Vec2::ZERO);
		let short_avail = available.min_element();
		if short_avail < MIN_WATER_RADIUS * 2.0 {
			return None;
		}

		let max_water_short = (short_room * 0.5).min(short_avail * 0.45);
		// Match authored hydro rim ([`TARGET_RIM_WIDTH`]) so index extents fit.
		let rim_claim = TARGET_RIM_WIDTH
			.min(short_avail * 0.35)
			.max(params.rim_width_min.clamp(0.2, TARGET_RIM_WIDTH));
		if short_avail < rim_claim + MIN_WATER_RADIUS * 2.0 {
			return None;
		}
		let _ = rim_u01; // reserved for later rim undershoot once extents are stable
		let rim = rim_claim;
		let apron = (short_avail * params.apron_frac.max(0.05))
			.max(short_avail * 0.14)
			.min(short_avail * 0.72);

		let leftover = Vec2::new(
			(available.x - rim_claim - apron).max(0.0),
			(available.y - rim_claim - apron).max(0.0),
		);
		let long_frac = params.long_axis_frac.clamp(0.45, 0.95);
		let leftover = Vec2::new(
			if available.x <= available.y {
				leftover.x.min(max_water_short)
			} else {
				leftover.x.min(available.x * long_frac)
			},
			if available.y <= available.x {
				leftover.y.min(max_water_short)
			} else {
				leftover.y.min(available.y * long_frac)
			},
		);

		let size_hi = params.water_scale.clamp(0.05, 1.0);
		let size_lo = params.water_scale_min.clamp(0.05, size_hi);
		let size_frac = size_lo + (size_hi - size_lo) * water_u01.clamp(0.0, 1.0);

		let circ = leftover.min_element() * size_frac;
		let full = leftover * size_frac;
		let aspect = aspect_blend(params, short_room, aspect_u01);
		let mut water = Vec2::new(circ + (full.x - circ) * aspect, circ + (full.y - circ) * aspect);
		if water.min_element() < MIN_WATER_RADIUS {
			return None;
		}
		if 2.0 * short_room < 2.0 * (2.0 * water.min_element()) * 0.99 {
			return None;
		}

		let rotation = rotation_u11.clamp(-1.0, 1.0) * params.rotation_amp.max(0.0);
		let band = rim + apron;
		let fit_radii = water + Vec2::splat(band);
		let (s, c) = rotation.sin_cos();
		let aabb = Vec2::new(
			((fit_radii.x * c).abs().powi(2) + (fit_radii.y * s).abs().powi(2)).sqrt(),
			((fit_radii.x * s).abs().powi(2) + (fit_radii.y * c).abs().powi(2)).sqrt(),
		);
		let scale = (available / aabb.max(Vec2::splat(1e-3))).min_element().clamp(0.0, 1.0);
		water *= scale;
		if water.min_element() < MIN_WATER_RADIUS {
			return None;
		}

		Some(Self {
			water_radii: water,
			rotation,
			rim_width: rim,
			apron_width: apron,
			plateau_radii: water + Vec2::splat(rim),
			mu,
		})
	}

	/// Axis-aligned helper used by unit tests (centroid = leaf center, no rotation).
	pub fn try_from_short_half(
		short_half: f32,
		params: LakeParams,
		water_u01: f32,
		rim_u01: f32,
	) -> Option<Self> {
		let s = short_half.max(0.0);
		let bounds = Bounds2::from_xz(-s, -s, s, s);
		Self::try_inscribed(bounds, Vec2::ZERO, params, water_u01, rim_u01, 0.0, 0.0)
	}
}

/// Aspect blend ∈ `[0, 1]` — weak on small leaves, strong on large.
pub(crate) fn aspect_blend(params: LakeParams, short_room: f32, aspect_u01: f32) -> f32 {
	let u = aspect_u01.clamp(0.0, 1.0);
	let ratio = (short_room / params.aspect_scale_ref.max(1.0)).max(1.0e-3);
	let t = (1.0 - 1.0 / ratio.max(1.0).sqrt()).clamp(0.0, 1.0);
	let small = params.aspect_small.clamp(0.0, 1.0);
	let strength = params.aspect_strength.clamp(0.0, 1.0) * (small + (1.0 - small) * t);
	let floor = (params.aspect_floor.clamp(0.0, 0.95) * t).clamp(0.0, 0.9);
	(strength * (floor + (1.0 - floor) * u)).clamp(0.0, 1.0)
}

/// Mid-rim characteristic radius used when surveying surrounding terrain.
pub(crate) fn shelf_survey_radius(budget: &LakeBandBudget) -> f32 {
	(budget.water_radius() + budget.rim_width * 0.5).max(1.0)
}
