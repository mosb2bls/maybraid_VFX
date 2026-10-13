//! Shared angular kit sizes, continuous-arc decomposition, and ring locus.

use bevy_math::Vec2;

/// Normalized angular kit sizes from the partition README.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArcKit {
	D180,
	D90,
	D15,
}

impl ArcKit {
	pub fn degrees(self) -> f32 {
		match self {
			Self::D180 => 180.0,
			Self::D90 => 90.0,
			Self::D15 => 15.0,
		}
	}
}

/// Outward XZ direction of an arc kit after Bevy YXZ yaw `radians`.
///
/// Rough-stone arc assets are authored on local \(+X\) (unit radius, center at
/// origin) and sweep toward \(+Z\). After yaw \(\phi\):
/// \(\hat{r} = (\cos\phi,\,-\sin\phi)\).
///
/// Use this (not a hard-coded \(−X\) locus) whenever a consumer maps plan angles
/// onto kit placement yaw.
#[inline]
pub fn arc_ring_dir(yaw_radians: f32) -> Vec2 {
	let (s, c) = yaw_radians.sin_cos();
	Vec2::new(c, -s)
}

/// [`arc_ring_dir`] with yaw in degrees.
#[inline]
pub fn arc_ring_dir_deg(yaw_degrees: f32) -> Vec2 {
	arc_ring_dir(yaw_degrees.to_radians())
}

/// Greedy decomposition of a continuous arc sweep into kit pieces.
///
/// Returns `(kit, yaw_offset_radians)`. Example: 45° → three [`ArcKit::D15`].
/// Offsets increase along the authored sweep so a parent placement yaw is the
/// **start** of the first piece.
pub fn decompose_arc_sweep(sweep_degrees: f32) -> Vec<(ArcKit, f32)> {
	let sign = if sweep_degrees < 0.0 { -1.0 } else { 1.0 };
	let mut remaining = sweep_degrees.abs();
	let mut cursor_deg = 0.0_f32;
	let mut out = Vec::new();
	const EPS: f32 = 1e-3;

	while remaining > EPS {
		let kit = if remaining + EPS >= 180.0 {
			ArcKit::D180
		} else if remaining + EPS >= 90.0 {
			ArcKit::D90
		} else {
			ArcKit::D15
		};
		let piece = kit.degrees();
		out.push((kit, f32::to_radians(sign * cursor_deg)));
		cursor_deg += piece;
		remaining -= piece;
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forty_five_degrees_is_three_fifteens() -> anyhow::Result<()> {
		let pieces = decompose_arc_sweep(45.0);
		assert_eq!(pieces.len(), 3);
		assert!(pieces.iter().all(|(k, _)| *k == ArcKit::D15));
		Ok(())
	}

	#[test]
	fn one_eighty_is_single_piece() -> anyhow::Result<()> {
		let pieces = decompose_arc_sweep(180.0);
		assert_eq!(pieces, vec![(ArcKit::D180, 0.0)]);
		Ok(())
	}

	#[test]
	fn ring_dir_yaw_zero_is_plus_x() -> anyhow::Result<()> {
		let d = arc_ring_dir(0.0);
		assert!((d.x - 1.0).abs() < 1e-5 && d.y.abs() < 1e-5, "{d:?}");
		Ok(())
	}
}
