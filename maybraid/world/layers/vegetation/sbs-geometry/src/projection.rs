//! Shared height-fraction projection profiles for stalk-and-ball-stick trees.

/// Friend's / Temperate Conifer logarithmic rounding ([RFC §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
///
/// \(\ell(u) = \ell_{\max} + (\ell_{\min} - \ell_{\max})\,\mathrm{falloff}(u)\) with
/// \(\mathrm{falloff}(u) = \ln(1 + \alpha u^\beta) / \ln(1 + \alpha)\).
pub fn logarithmic_rounding_projection(
	ell_max: f32,
	ell_min: f32,
	u: f32,
	alpha: f32,
	beta: f32,
) -> f32 {
	let u = u.clamp(0.0, 1.0);
	let alpha = alpha.max(1e-6);
	let denom = (1.0 + alpha).ln();
	let falloff = if denom.abs() < 1e-12 { u } else { (1.0 + alpha * u.powf(beta)).ln() / denom };
	ell_max + (ell_min - ell_max) * falloff
}

/// Bounded logit vase profile over normalized ring height ([RFC §3.1.7.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/03-vase-tree/README.md)).
pub fn vase_profile(u: f32, eps: f32, center: f32) -> f32 {
	let eps = eps.clamp(1e-4, 0.49);
	let u = u.clamp(eps, 1.0 - eps);
	let center = center.clamp(eps, 1.0 - eps);

	let steepness = ((1.0 - eps) / eps).ln();
	let x = (u / (1.0 - u)).ln();
	let c = (center / (1.0 - center)).ln();

	((x - c) / (2.0 * steepness) + 0.5).clamp(0.0, 1.0)
}

/// World projection length from height `H` and min/max fractions mixed by [`vase_profile`].
pub fn vase_projection_length(
	height: f32,
	min_fraction_of_height: f32,
	max_fraction_of_height: f32,
	u: f32,
	eps: f32,
	center: f32,
) -> f32 {
	let h = height.max(1e-6);
	let t = vase_profile(u, eps, center);
	let f = min_fraction_of_height + (max_fraction_of_height - min_fraction_of_height) * t;
	h * f
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn endpoints_match_max_and_min() {
		let h = 30.0;
		let ell_max = 0.06 * h;
		let ell_min = 0.015 * h;
		let l0 = logarithmic_rounding_projection(ell_max, ell_min, 0.0, 8.0, 3.0);
		let l1 = logarithmic_rounding_projection(ell_max, ell_min, 1.0, 8.0, 3.0);
		assert!((l0 - ell_max).abs() < 1e-4);
		assert!((l1 - ell_min).abs() < 1e-4);
	}

	#[test]
	fn mid_canopy_stays_near_max_longer_than_linear() {
		let h = 30.0;
		let ell_max = 0.06 * h;
		let ell_min = 0.015 * h;
		let u = 0.5;
		let log = logarithmic_rounding_projection(ell_max, ell_min, u, 8.0, 3.0);
		let linear = ell_max + (ell_min - ell_max) * u;
		assert!(log > linear, "log profile should delay falloff: log={log} linear={linear}");
	}

	#[test]
	fn vase_profile_endpoints_are_zero_and_one() {
		assert!((vase_profile(0.0, 0.08, 0.5) - 0.0).abs() < 1e-3);
		assert!((vase_profile(1.0, 0.08, 0.5) - 1.0).abs() < 1e-3);
	}

	#[test]
	fn vase_projection_widens_toward_rim() {
		let h = 30.0;
		let low = vase_projection_length(h, 0.10, 0.45, 0.0, 0.08, 0.5);
		let high = vase_projection_length(h, 0.10, 0.45, 1.0, 0.08, 0.5);
		assert!(high > low, "rim should project farther: low={low} high={high}");
	}
}
