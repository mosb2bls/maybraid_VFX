//! Geom-free placement and optional geometry+pose pairing.

use bevy_math::{EulerRot, Quat, Vec3};

/// Translation / yaw / pitch / roll / scale for a kit piece or continuous form in cell space.
///
/// Partition / floor / door kits are authored in a **normalized** local space
/// (rectangle panels: ground \(X,Z \in [0, 1]\), \(Y \in [-0.2, 0.2]\); angular arcs:
/// radius \(1\), full height \(Y \in [0, 1]\); slices \(Y \in [0, 0.2]\)). Buildings
/// map that kit into cell space via [`Self::scale`] (and pitch for standing panels).
///
/// Rotation order (intrinsic [`EulerRot::YXZ`]):
/// - **yaw** about world \(+Y\) — plan facing
/// - **pitch** about local \(+X\) — stand a ground panel as a wall (\(\pi/2\))
/// - **roll** about local \(+Z\) — reserved; polyline walls stay plumb (path \(Y\) carries slope)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
	/// Translation in cell-local space.
	pub translation: Vec3,
	/// Yaw about +Y (radians).
	pub yaw: f32,
	/// Pitch about local +X after yaw (radians). \(\pi/2\) stands a ground panel.
	pub pitch: f32,
	/// Roll about local +Z after yaw/pitch (radians). Unused by plumb partition walls.
	pub roll: f32,
	/// Non-uniform scale applied to the normalized kit before rotation.
	///
	/// For a circular wall of radius \(R\) and storey height \(H\), use
	/// `Vec3::new(R, H, R)`.
	pub scale: Vec3,
}

impl Placement {
	pub const IDENTITY: Self =
		Self { translation: Vec3::ZERO, yaw: 0.0, pitch: 0.0, roll: 0.0, scale: Vec3::ONE };

	pub fn new(translation: Vec3, yaw: f32) -> Self {
		Self { translation, yaw, pitch: 0.0, roll: 0.0, scale: Vec3::ONE }
	}

	pub fn at_origin() -> Self {
		Self::IDENTITY
	}

	pub fn with_pitch(mut self, pitch: f32) -> Self {
		self.pitch = pitch;
		self
	}

	pub fn with_roll(mut self, roll: f32) -> Self {
		self.roll = roll;
		self
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.scale = scale;
		self
	}

	pub fn rotation(self) -> Quat {
		Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, self.roll)
	}

	/// Compose a child placement under this parent (scale → rotate → translate).
	pub fn compose_child(self, child: Placement) -> Placement {
		Placement {
			translation: self.translation + self.rotation() * (child.translation * self.scale),
			yaw: self.yaw + child.yaw,
			pitch: self.pitch + child.pitch,
			roll: self.roll + child.roll,
			scale: self.scale * child.scale,
		}
	}
}

impl Default for Placement {
	fn default() -> Self {
		Self::IDENTITY
	}
}

/// Geometry paired with a [`Placement`] (migration / internal tessellation aid).
#[derive(Debug, Clone, PartialEq)]
pub struct Placed<G> {
	pub geom: G,
	pub placement: Placement,
}

impl<G> Placed<G> {
	pub fn new(geom: G, translation: Vec3, yaw: f32) -> Self {
		Self { geom, placement: Placement::new(translation, yaw) }
	}

	pub fn at_origin(geom: G) -> Self {
		Self { geom, placement: Placement::at_origin() }
	}

	pub fn with_placement(geom: G, placement: Placement) -> Self {
		Self { geom, placement }
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.placement = self.placement.with_scale(scale);
		self
	}

	pub fn map_geom<H>(self, f: impl FnOnce(G) -> H) -> Placed<H> {
		Placed { geom: f(self.geom), placement: self.placement }
	}

	pub fn translation(&self) -> Vec3 {
		self.placement.translation
	}

	pub fn yaw(&self) -> f32 {
		self.placement.yaw
	}

	pub fn pitch(&self) -> f32 {
		self.placement.pitch
	}

	pub fn roll(&self) -> f32 {
		self.placement.roll
	}

	pub fn scale(&self) -> Vec3 {
		self.placement.scale
	}
}
