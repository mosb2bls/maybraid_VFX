//! Plan / slope orientation helpers shared by panels and thin-wall polylines.

/// Minimum plan or slope kink (radians) before a joint is emitted on thin-wall polylines.
pub const DEFAULT_MIN_JOINT_ANGLE: f32 = 0.1;

/// Plan yaw for a horizontal displacement \((\mathrm{d}x, \mathrm{d}z)\).
pub fn yaw_along_xz(dx: f32, dz: f32) -> f32 {
	(-dz).atan2(dx)
}

/// Slope angle of an edge vs horizontal (joint kink sizing / incoming cues).
pub fn roll_along_slope(dx: f32, dy: f32, dz: f32) -> f32 {
	let horiz = (dx * dx + dz * dz).sqrt();
	dy.atan2(horiz.max(1e-8))
}

pub(crate) fn wrap_pi(mut a: f32) -> f32 {
	while a > std::f32::consts::PI {
		a -= std::f32::consts::TAU;
	}
	while a < -std::f32::consts::PI {
		a += std::f32::consts::TAU;
	}
	a
}
