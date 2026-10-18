//! Approximate terrain pitch and roll from a rest support span. No IK.
//!
//! Sample ground at front / hind / left / right, take `atan(Δh / run)`, and
//! apply separate pitch and roll weights. Mesh faces `+Z`; positive local `X`
//! dips the nose, so sagittal slope is negated. Family roll weight is 0
//! (stand upright); set [`TerrainPitch::roll_weight`] to bank. The capsule
//! stays upright and owns world Y. Quadruped rays follow the live shoulder–hip
//! axis when girdles exist, not Bevy `+Z`, so a long body still measures the
//! slope it is standing on. Rest wheelbase locks on the first good measure so a
//! pitched chord cannot shrink the next `atan`. Sample the **pitched** footprint
//! so the visual chord matches that slope without max-lifting the rear into the
//! air.

use bevy::prelude::*;

use crate::rig::RigSkeletonKind;

/// Ground-plane heading owned by locomotion, look, or combat.
///
/// Terrain pitch reads this source value instead of trying to recover yaw from
/// the tilted [`Transform`] that it wrote on the previous frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct CharacterHeading(pub Vec3);

/// Ignore sub-degree changes in movement direction.
pub const HEADING_DEADZONE: f32 = 0.01;
/// Maximum body-heading turn rate in radians per second.
pub const HEADING_TURN_RATE: f32 = 5.5;

impl CharacterHeading {
	pub fn from_rotation(rotation: Quat) -> Self {
		Self(xz_dir(rotation * Vec3::Z).unwrap_or(Vec3::Z))
	}

	/// Heading on the ground plane, initializing an unset heading from `visual` without writing.
	///
	/// A stored planar heading is returned as-is so a second `normalize` cannot drift by an ulp
	/// and mark [`CharacterHeading`] changed every frame.
	pub fn facing(&self, visual: &Transform) -> Vec3 {
		if self.0.y == 0.0 && self.0.length_squared() >= 1e-6 {
			return self.0;
		}
		xz_dir(self.0).or_else(|| xz_dir(-*visual.forward())).unwrap_or(Vec3::Z)
	}

	/// Return a normalized heading, initializing an unset heading from `visual`.
	pub fn resolve(&mut self, visual: &Transform) -> Vec3 {
		let facing = self.facing(visual);
		if self.0 != facing {
			self.0 = facing;
		}
		facing
	}

	/// Whether [`Self::turn_toward`] would write heading or visual this frame.
	pub fn would_turn_toward(&self, visual: &Transform, wish: Vec3) -> bool {
		let Some(target) = xz_dir(wish) else {
			return false;
		};
		self.facing(visual).angle_between(target) >= HEADING_DEADZONE
	}

	/// Store `facing` and write its yaw to the visual.
	pub fn set(&mut self, visual: &mut Transform, facing: Vec3) {
		let Some(facing) = xz_dir(facing) else {
			return;
		};
		self.0 = facing;
		visual.look_to(-facing, Vec3::Y);
	}

	/// Slerp this ground-plane heading toward a movement wish.
	pub fn turn_toward(&mut self, visual: &mut Transform, wish: Vec3, dt: f32) {
		let current = self.facing(visual);
		let Some(target) = xz_dir(wish) else {
			return;
		};
		let angle = current.angle_between(target);
		if angle < HEADING_DEADZONE {
			return;
		}
		let t = (HEADING_TURN_RATE * dt / angle).min(1.0);
		self.set(visual, current.slerp(target, t));
	}
}

/// Last probe locations for gizmos. Written by [`crate::elevation::apply_terrain_pitch`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainPitchProbe {
	pub origin: Vec3,
	pub front: Vec3,
	pub hind: Vec3,
	pub front_hit: bool,
	pub hind_hit: bool,
	/// Explicit ground-plane character heading.
	pub visual_facing: Vec3,
	/// Axis actually used for front/hind rays.
	pub sample_facing: Vec3,
}

/// Live girdle world points from prepare. `None` means the bone was not in the map.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainPitchGirdles {
	pub shoulder_l: Option<Vec3>,
	pub shoulder_r: Option<Vec3>,
	pub hip_l: Option<Vec3>,
	pub hip_r: Option<Vec3>,
	pub front: Option<Vec3>,
	pub hind: Option<Vec3>,
	/// True when [`sagittal_axis`] accepted the front/hind midpoints.
	pub sagittal_ok: bool,
}

#[derive(Clone, Copy, Debug)]
struct GirdleSnapshot {
	girdles: TerrainPitchGirdles,
	sagittal: Option<Vec3>,
	half_span: Option<f32>,
	half_width: Option<f32>,
}

/// Visual tilt plus the rest support span used to sample terrain.
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainPitch {
	pub half_span: f32,
	pub half_width: f32,
	pub pitch_weight: f32,
	/// Fraction of observed side slope. Family default is 0; set per species to bank.
	pub roll_weight: f32,
	/// Local-X radians (nose down is positive).
	pub pitch: f32,
	/// Local-Z radians (right side up is positive).
	pub roll: f32,
	/// Smoothed local Y for a capsule child. Ignored for world-placed hosts.
	pub support: f32,
	/// Unit XZ hind→front from live girdles. [`Vec3::ZERO`] until measured.
	pub sagittal: Vec3,
	/// Rest wheelbase is frozen after the first good girdle measure.
	pub support_locked: bool,
	pub probe: TerrainPitchProbe,
	pub girdles: TerrainPitchGirdles,
}

impl TerrainPitch {
	pub fn new(kind: RigSkeletonKind, half_span: f32, half_width: f32) -> Self {
		Self {
			half_span,
			half_width,
			pitch_weight: kind.default_pitch_weight(),
			roll_weight: kind.default_roll_weight(),
			pitch: 0.0,
			roll: 0.0,
			support: 0.0,
			sagittal: Vec3::ZERO,
			support_locked: false,
			probe: TerrainPitchProbe::default(),
			girdles: TerrainPitchGirdles::default(),
		}
	}

	/// Whether [`Self::record_girdles`] would write any stored field.
	pub fn would_record_girdles(
		&self,
		shoulder_l: Option<Vec3>,
		shoulder_r: Option<Vec3>,
		hip_l: Option<Vec3>,
		hip_r: Option<Vec3>,
	) -> bool {
		self.girdle_write(Self::girdle_snapshot(shoulder_l, shoulder_r, hip_l, hip_r))
	}

	/// Store live girdle world points for gizmos. Rest [`Self::half_span`] /
	/// [`Self::half_width`] lock on the first long enough shoulder–hip run so a
	/// pitched chord cannot shrink the next sample. [`Self::sagittal`] stays live
	/// so yaw can turn the probe axis.
	pub fn record_girdles(
		&mut self,
		shoulder_l: Option<Vec3>,
		shoulder_r: Option<Vec3>,
		hip_l: Option<Vec3>,
		hip_r: Option<Vec3>,
	) {
		self.apply_girdle_snapshot(Self::girdle_snapshot(shoulder_l, shoulder_r, hip_l, hip_r));
	}

	fn girdle_snapshot(
		shoulder_l: Option<Vec3>,
		shoulder_r: Option<Vec3>,
		hip_l: Option<Vec3>,
		hip_r: Option<Vec3>,
	) -> GirdleSnapshot {
		let front = Self::girdle_midpoint([shoulder_l, shoulder_r].into_iter().flatten());
		let hind = Self::girdle_midpoint([hip_l, hip_r].into_iter().flatten());
		let left = Self::girdle_midpoint([shoulder_l, hip_l].into_iter().flatten());
		let right = Self::girdle_midpoint([shoulder_r, hip_r].into_iter().flatten());
		let sagittal = match (front, hind) {
			(Some(f), Some(h)) => Self::sagittal_axis(f, h),
			_ => None,
		};
		GirdleSnapshot {
			girdles: TerrainPitchGirdles {
				shoulder_l,
				shoulder_r,
				hip_l,
				hip_r,
				front,
				hind,
				sagittal_ok: sagittal.is_some(),
			},
			sagittal,
			half_span: Self::measured_support_half(front, hind),
			half_width: Self::measured_support_half(left, right),
		}
	}

	fn girdle_write(&self, snapshot: GirdleSnapshot) -> bool {
		if self.girdles != snapshot.girdles {
			return true;
		}
		let Some(axis) = snapshot.sagittal else {
			return false;
		};
		self.sagittal != axis || !self.support_locked
	}

	fn apply_girdle_snapshot(&mut self, snapshot: GirdleSnapshot) {
		if let Some(axis) = snapshot.sagittal {
			self.sagittal = axis;
			if !self.support_locked {
				if let Some(span) = snapshot.half_span {
					self.half_span = span;
				}
				if let Some(width) = snapshot.half_width {
					self.half_width = width;
				}
				self.support_locked = true;
			}
		}
		self.girdles = snapshot.girdles;
	}

	/// Midpoint of named bones in XZ, if at least one exists.
	pub fn girdle_midpoint(positions: impl IntoIterator<Item = Vec3>) -> Option<Vec3> {
		let mut sum = Vec3::ZERO;
		let mut n = 0u32;
		for p in positions {
			sum += p;
			n += 1;
		}
		(n > 0).then(|| sum / n as f32)
	}

	/// XZ half-distance between two support samples, if they are far enough apart.
	pub fn measured_support_half(a: Option<Vec3>, b: Option<Vec3>) -> Option<f32> {
		let (Some(a), Some(b)) = (a, b) else {
			return None;
		};
		let delta = Vec2::new(a.x - b.x, a.z - b.z).length() * 0.5;
		(delta >= MIN_MEASURED).then_some(delta)
	}

	/// Hind→front on the ground plane from live girdle midpoints.
	pub fn sagittal_axis(front: Vec3, hind: Vec3) -> Option<Vec3> {
		xz_unit(front - hind)
	}

	/// Front/hind sample direction: live spine when girdles exist, else mesh `+Z`.
	pub fn sample_facing(&self, visual_facing: Vec3) -> Vec3 {
		xz_unit(self.sagittal).unwrap_or(visual_facing)
	}

	/// Local-X angle: nose up when the front sample is higher (`+Z` mesh).
	pub fn observed_pitch(front_height: f32, hind_height: f32, half_span: f32) -> f32 {
		-slope_angle(front_height, hind_height, half_span)
	}

	/// Local-Z angle: right side up when the right sample is higher.
	pub fn observed_roll(left_height: f32, right_height: f32, half_width: f32) -> f32 {
		slope_angle(right_height, left_height, half_width)
	}

	pub fn step_toward(current: f32, target: f32, dt: f32) -> f32 {
		Self::step_toward_rate(current, target, dt, TILT_RATE)
	}

	pub fn step_toward_rate(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
		let max_step = rate * dt;
		current + (target - current).clamp(-max_step, max_step)
	}

	/// Exponential blend toward `target`, then clamp to `rate`.
	pub fn follow_target(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
		let alpha = 1.0 - (-TILT_SMOOTH * dt.max(0.0)).exp();
		let blended = current + (target - current) * alpha;
		Self::step_toward_rate(current, blended, dt, rate)
	}

	/// Yaw from flattened facing (`look_to(-facing)`), then local pitch and roll.
	pub fn facing_with_tilt(&self, facing_xz: Vec3) -> Quat {
		self.facing_with_support_tilt(facing_xz, facing_xz)
	}

	/// Compose explicit heading with pitch about the axis perpendicular to `sagittal`.
	///
	/// When the shoulder–hip axis is not mesh `+Z`, local `X` would bank the spine
	/// instead of planting it. World lateral is `Y × sagittal`, brought into the
	/// yawed frame so a +X spine still pitches while wish `+Z` stays level.
	pub fn facing_with_support_tilt(&self, yaw_facing: Vec3, sagittal: Vec3) -> Quat {
		let yaw_facing = Vec3::new(yaw_facing.x, 0.0, yaw_facing.z);
		let yaw = if yaw_facing.length_squared() < 1e-6 {
			Quat::IDENTITY
		} else {
			Transform::IDENTITY.looking_to(-yaw_facing, Vec3::Y).rotation
		};
		let spine = Vec3::new(sagittal.x, 0.0, sagittal.z);
		let spine = if spine.length_squared() < 1e-6 { yaw_facing } else { spine };
		let lateral = Vec3::Y.cross(spine);
		let pitch_q = if lateral.length_squared() < 1e-6 {
			Quat::IDENTITY
		} else {
			Quat::from_axis_angle(yaw.inverse() * lateral.normalize(), self.pitch)
		};
		yaw * pitch_q * Quat::from_rotation_z(self.roll)
	}

	/// Horizontal half-run of the rest chord after pitch (girdles move closer in XZ).
	pub fn pitched_half_run(&self, pitch: f32) -> f32 {
		(self.half_span * pitch.cos().abs()).max(MIN_MEASURED)
	}

	/// Signed visual Y so the chord midpoint matches the average of the front/hind
	/// samples. Zero on a plane when those samples are taken at [`Self::pitched_half_run`].
	pub fn support_offset(center_height: f32, front_height: f32, hind_height: f32) -> f32 {
		((front_height + hind_height) * 0.5 - center_height)
			.clamp(-MAX_SUPPORT_OFFSET, MAX_SUPPORT_OFFSET)
	}
}

/// Match Durham / playground walkable slopes so long bodies can follow the mesh.
pub const MAX_TILT: f32 = 80.0_f32.to_radians();
/// Max rad/s toward the sampled tilt. Large changes still take several frames.
pub const TILT_RATE: f32 = 3.0;
/// Exponential follow rate (1/s) so sub-rate-cap noise is low-passed, not copied.
pub const TILT_SMOOTH: f32 = 4.0;
/// Max m/s toward the sampled support offset.
pub const SUPPORT_RATE: f32 = 0.8;

const HUMANOID_HALF_SPAN: f32 = 0.22;
const QUADRUPED_HALF_SPAN: f32 = 1.2;
const FORELIMBED_HALF_SPAN: f32 = 0.4;
const HUMANOID_HALF_WIDTH: f32 = 0.18;
const QUADRUPED_HALF_WIDTH: f32 = 0.45;
const FORELIMBED_HALF_WIDTH: f32 = 0.25;
const MIN_MEASURED: f32 = 0.12;
/// Cap on signed visual Y so a bad ray cannot yank the mesh through the world.
const MAX_SUPPORT_OFFSET: f32 = 2.0;

pub const QUADRUPED_FRONT: &[&str] = &["shoulder.L", "shoulder.R"];
pub const QUADRUPED_HIND: &[&str] = &["hip.L", "hip.R"];
pub const QUADRUPED_LEFT: &[&str] = &["shoulder.L", "hip.L"];
pub const QUADRUPED_RIGHT: &[&str] = &["shoulder.R", "hip.R"];

impl RigSkeletonKind {
	/// Rest front/hind wheelbase when girdles are missing or stacked.
	pub fn default_half_span(self) -> f32 {
		match self {
			Self::Humanoid | Self::Neck => HUMANOID_HALF_SPAN,
			Self::Quadruped => QUADRUPED_HALF_SPAN,
			Self::Forelimbed => FORELIMBED_HALF_SPAN,
		}
	}

	/// Rest left/right stance width when girdles are missing or stacked.
	pub fn default_half_width(self) -> f32 {
		match self {
			Self::Humanoid | Self::Neck => HUMANOID_HALF_WIDTH,
			Self::Quadruped => QUADRUPED_HALF_WIDTH,
			Self::Forelimbed => FORELIMBED_HALF_WIDTH,
		}
	}

	/// How much of the front/hind slope to apply. Long bodies need more or they sink.
	pub fn default_pitch_weight(self) -> f32 {
		match self {
			Self::Humanoid | Self::Neck => 0.4,
			Self::Quadruped => 1.0,
			Self::Forelimbed => 0.7,
		}
	}

	/// How much of the left/right slope to apply. Zero: stand upright; opt in later.
	pub fn default_roll_weight(self) -> f32 {
		0.0
	}

	/// Rest wheelbase from girdle world positions, or the family default.
	pub fn half_span_from_girdles(self, front: Option<Vec3>, hind: Option<Vec3>) -> f32 {
		measured_half(front, hind, self.default_half_span())
	}

	/// Rest stance width from left/right world positions, or the family default.
	pub fn half_width_from_sides(self, left: Option<Vec3>, right: Option<Vec3>) -> f32 {
		measured_half(left, right, self.default_half_width())
	}
}

fn measured_half(a: Option<Vec3>, b: Option<Vec3>, fallback: f32) -> f32 {
	TerrainPitch::measured_support_half(a, b).unwrap_or(fallback)
}

/// Flattened unit XZ, or `None` if the vector has no ground-plane direction.
pub fn xz_dir(v: Vec3) -> Option<Vec3> {
	let xz = Vec3::new(v.x, 0.0, v.z);
	(xz.length_squared() >= 1e-6).then(|| xz.normalize())
}

/// Flattened unit XZ, or `None` if the vector is too short to be a support axis.
pub fn xz_unit(v: Vec3) -> Option<Vec3> {
	let xz = Vec3::new(v.x, 0.0, v.z);
	(xz.length() >= 2.0 * MIN_MEASURED).then(|| xz.normalize())
}

fn slope_angle(high_side: f32, low_side: f32, half_run: f32) -> f32 {
	let run = (2.0 * half_run).max(1e-3);
	((high_side - low_side) / run).atan().clamp(-MAX_TILT, MAX_TILT)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uphill_front_pitches_nose_up() {
		let pitch_angle = TerrainPitch::observed_pitch(2.0, 1.0, 0.5);
		assert!(pitch_angle < 0.0);
		assert!(pitch_angle.abs() <= MAX_TILT);
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Humanoid, 0.5, 0.18);
		pitch.pitch = pitch_angle;
		let nose = pitch.facing_with_tilt(Vec3::Z) * Vec3::Z;
		assert!(nose.y > 0.0, "uphill should raise mesh +Z, y={}", nose.y);
	}

	#[test]
	fn high_right_rolls_right_up() {
		let roll = TerrainPitch::observed_roll(1.0, 2.0, 0.5);
		assert!(roll > 0.0);
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Humanoid, 0.5, 0.18);
		pitch.roll = roll;
		let right = pitch.facing_with_tilt(Vec3::Z) * Vec3::X;
		assert!(right.y > 0.0, "high right should raise mesh +X, y={}", right.y);
	}

	#[test]
	fn support_tilt_pitches_a_plus_x_spine_without_banking_plus_z() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Humanoid, 0.5, 0.18);
		pitch.pitch = -0.6;
		let q = pitch.facing_with_support_tilt(Vec3::Z, Vec3::X);
		let spine = q * Vec3::X;
		assert!(spine.y > 0.0, "uphill along +X should raise mesh +X, y={}", spine.y);
		let nose = q * Vec3::Z;
		assert!(nose.y.abs() < 0.05, "wish +Z should stay level, y={}", nose.y);
	}

	#[test]
	fn flat_ground_is_zero() {
		assert_eq!(TerrainPitch::observed_pitch(3.0, 3.0, 0.9), 0.0);
		assert_eq!(TerrainPitch::observed_roll(3.0, 3.0, 0.45), 0.0);
	}

	#[test]
	fn missing_girdles_use_family_default() {
		assert_eq!(
			RigSkeletonKind::Quadruped.half_span_from_girdles(None, None),
			QUADRUPED_HALF_SPAN
		);
		assert_eq!(
			RigSkeletonKind::Quadruped.half_width_from_sides(None, None),
			QUADRUPED_HALF_WIDTH
		);
	}

	#[test]
	fn stacked_girdles_use_family_default() {
		let origin = Vec3::new(10.0, 1.0, 4.0);
		assert_eq!(
			RigSkeletonKind::Quadruped.half_span_from_girdles(Some(origin), Some(origin)),
			QUADRUPED_HALF_SPAN
		);
	}

	#[test]
	fn measured_girdles_win_when_separated() {
		let front = Vec3::new(0.0, 1.0, 2.0);
		let hind = Vec3::new(0.0, 1.0, 0.0);
		assert!(
			(RigSkeletonKind::Quadruped.half_span_from_girdles(Some(front), Some(hind)) - 1.0)
				.abs() < 1e-5
		);
	}

	#[test]
	fn offset_is_zero_on_a_plane() {
		assert!(TerrainPitch::support_offset(0.0, 0.2, -0.2).abs() < 1e-5);
		assert!(TerrainPitch::support_offset(1.2, 2.4, 0.0).abs() < 1e-5);
	}

	#[test]
	fn pitched_run_shrinks_with_tilt() {
		let span = 1.2;
		let pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, span, 0.45);
		assert!((pitch.pitched_half_run(0.0) - span).abs() < 1e-5);
		assert!((pitch.pitched_half_run(60.0_f32.to_radians()) - span * 0.5).abs() < 1e-5);
	}

	#[test]
	fn plane_slope_survives_resampling_at_pitched_run() {
		let alpha = 40.0_f32.to_radians();
		let span = 1.2;
		let tan = alpha.tan();
		let pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, span, 0.45);
		let coarse = TerrainPitch::observed_pitch(tan * span, -tan * span, span);
		assert!((coarse + alpha).abs() < 1e-4);
		let run = pitch.pitched_half_run(coarse);
		let refined = TerrainPitch::observed_pitch(tan * run, -tan * run, run);
		assert!((refined - coarse).abs() < 1e-4);
		assert!(TerrainPitch::support_offset(0.0, tan * run, -tan * run).abs() < 1e-5);
	}

	#[test]
	fn support_offset_raises_over_a_dip() {
		let offset = TerrainPitch::support_offset(0.0, 1.0, 1.0);
		assert!((offset - 1.0).abs() < 1e-5);
	}

	#[test]
	fn measured_support_ignores_stacked_girdles() {
		let origin = Vec3::new(10.0, 1.0, 4.0);
		assert_eq!(TerrainPitch::measured_support_half(Some(origin), Some(origin)), None);
	}

	#[test]
	fn family_roll_weight_defaults_to_zero() {
		for kind in [
			RigSkeletonKind::Humanoid,
			RigSkeletonKind::Quadruped,
			RigSkeletonKind::Forelimbed,
			RigSkeletonKind::Neck,
		] {
			assert_eq!(kind.default_roll_weight(), 0.0);
			assert!(kind.default_pitch_weight() > 0.0);
		}
	}

	#[test]
	fn sagittal_axis_follows_shoulder_to_hip() {
		let front = Vec3::new(3.0, 2.0, 1.0);
		let hind = Vec3::new(1.0, 0.5, 1.0);
		let axis = TerrainPitch::sagittal_axis(front, hind).expect("separated girdles");
		assert!((axis - Vec3::X).length() < 1e-5);
		assert!(TerrainPitch::sagittal_axis(front, front).is_none());
	}

	#[test]
	fn sample_facing_prefers_girdle_axis_over_mesh_plus_z() {
		let visual = Vec3::Z;
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		assert_eq!(pitch.sample_facing(visual), visual);
		pitch.sagittal = Vec3::X;
		let spine = pitch.sample_facing(visual);
		assert!((spine - Vec3::X).length() < 1e-5);
	}

	#[test]
	fn heading_resolves_initial_visual_rotation() {
		let rotation = Quat::from_rotation_y(0.7);
		let visual = Transform::from_rotation(rotation);
		let mut heading = CharacterHeading::default();
		assert!((heading.resolve(&visual) - rotation * Vec3::Z).length() < 1e-5);
	}

	#[test]
	fn heading_resolve_does_not_rewrite_an_already_set_facing() {
		let visual = Transform::from_rotation(Quat::from_rotation_y(0.7));
		let mut heading = CharacterHeading::from_rotation(visual.rotation);
		let before = heading.0;
		heading.resolve(&visual);
		assert_eq!(heading.0, before);
		assert!(!heading.would_turn_toward(&visual, Vec3::ZERO));
		assert!(!heading.would_turn_toward(&visual, heading.0));
	}

	#[test]
	fn heading_accepts_small_real_turns() {
		let mut visual = Transform::IDENTITY;
		let mut heading = CharacterHeading::from_rotation(visual.rotation);
		let target = Quat::from_rotation_y(0.02) * Vec3::Z;
		heading.turn_toward(&mut visual, target, 1.0 / 120.0);
		assert!((heading.0 - target).length() < 1e-5);
		assert!((-*visual.forward() - target).length() < 1e-5);
	}

	#[test]
	fn record_girdles_accepts_a_long_xz_wheelbase() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		pitch.record_girdles(
			Some(Vec3::new(0.0, 1.0, 2.0)),
			Some(Vec3::new(0.0, 1.0, 2.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
		);
		assert!(pitch.girdles.sagittal_ok);
		assert!((pitch.half_span - 1.0).abs() < 1e-5);
		assert!((pitch.sagittal - Vec3::Z).length() < 1e-5);
		assert!(pitch.support_locked);
	}

	#[test]
	fn record_girdles_rejects_stacked_xz() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		let a = Vec3::new(1.0, 0.0, 1.0);
		let b = Vec3::new(1.0, 4.0, 1.0);
		pitch.record_girdles(Some(a), Some(a), Some(b), Some(b));
		assert!(pitch.girdles.front.is_some());
		assert!(!pitch.girdles.sagittal_ok);
		assert_eq!(pitch.sagittal, Vec3::ZERO);
		assert!(!pitch.support_locked);
		assert!((pitch.half_span - 1.2).abs() < 1e-5);
	}

	#[test]
	fn record_girdles_locks_span_but_keeps_live_axis_and_gizmos() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		pitch.record_girdles(
			Some(Vec3::new(0.0, 1.0, 2.0)),
			Some(Vec3::new(0.0, 1.0, 2.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
		);
		pitch.record_girdles(
			Some(Vec3::new(4.0, 1.0, 0.0)),
			Some(Vec3::new(4.0, 1.0, 0.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
			Some(Vec3::new(0.0, 1.0, 0.0)),
		);
		assert!(pitch.support_locked);
		assert!((pitch.half_span - 1.0).abs() < 1e-5);
		assert!((pitch.sagittal - Vec3::X).length() < 1e-5);
		assert_eq!(pitch.girdles.front, Some(Vec3::new(4.0, 1.0, 0.0)));
	}

	#[test]
	fn settled_girdles_do_not_need_another_record() {
		let mut pitch = TerrainPitch::new(RigSkeletonKind::Quadruped, 1.2, 0.45);
		let front = Some(Vec3::new(0.0, 1.0, 2.0));
		let hind = Some(Vec3::new(0.0, 1.0, 0.0));
		pitch.record_girdles(front, front, hind, hind);
		assert!(!pitch.would_record_girdles(front, front, hind, hind));
	}

	#[test]
	fn follow_target_damps_small_error_and_caps_large() {
		let dt = 1.0 / 60.0;
		let small = TerrainPitch::follow_target(0.0, 0.02, dt, TILT_RATE);
		assert!(small > 0.0);
		assert!(small < 0.02);
		let large = TerrainPitch::follow_target(0.0, 2.0, dt, TILT_RATE);
		assert!((large - TILT_RATE * dt).abs() < 1e-5);
	}
}
