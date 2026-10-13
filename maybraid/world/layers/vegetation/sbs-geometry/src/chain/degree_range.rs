//! Direction blending and small-angle perturbation for growth hysteresis.

use bevy_math::Vec3;

/// Spherical blend between normalized `incoming` and `bias` with weight `t` in `[0, 1]`.
pub fn blend_direction(incoming_ray: Vec3, bias_ray: Vec3, t: f32) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	let a = incoming_ray.normalize_or_zero();
	let b = bias_ray.normalize_or_zero();
	(a * (1.0 - t) + b * t).normalize_or_zero()
}

/// Perturb unit direction `mean` by up to roughly `angular_scale * max(|u|, |v|)` in the tangent plane.
pub fn perturb_direction(mean: Vec3, angular_scale: f32, u: f32, v: f32) -> Vec3 {
	let m = mean.normalize_or_zero();
	if m.length_squared() < 1e-10 {
		return Vec3::Y;
	}
	let up = if m.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
	let t = m.cross(up).normalize_or_zero();
	let b = m.cross(t).normalize_or_zero();
	let u = u.clamp(-1.0, 1.0);
	let v = v.clamp(-1.0, 1.0);
	let offset = (t * u + b * v) * angular_scale;
	(m + offset).normalize_or_zero()
}
