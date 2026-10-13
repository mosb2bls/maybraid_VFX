//! Asset paths relative to the `maybraid/assets` root.

/// Runtime asset path relative to the Bevy asset root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetPath(&'static str);

impl AssetPath {
	pub const fn new(path: &'static str) -> Self {
		Self(path)
	}

	pub const fn as_str(self) -> &'static str {
		self.0
	}

	pub fn scene_ref(self) -> scene_ref::SceneRef {
		scene_ref::SceneRef::glb(self.0)
	}
}

impl std::fmt::Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

/// Stick GLBs under `vegetation/sticks/`.
pub mod sticks {
	use super::AssetPath;

	/// Kits under `vegetation/sticks/standard/`.
	pub mod standard {
		use super::AssetPath;

		pub const HIGH: AssetPath = AssetPath::new("vegetation/sticks/standard/001_high_res.glb");
		pub const MID: AssetPath = AssetPath::new("vegetation/sticks/standard/001_mid_res.glb");
		pub const LOW: AssetPath = AssetPath::new("vegetation/sticks/standard/001_low_res.glb");

		/// Trunk geometry variant (`trunk_001_*`) — same style kit, longer mesh-LOD lifetime.
		pub mod trunk {
			use super::AssetPath;

			pub const HIGH: AssetPath =
				AssetPath::new("vegetation/sticks/standard/trunk_001_high_res.glb");
			pub const MID: AssetPath =
				AssetPath::new("vegetation/sticks/standard/trunk_001_mid_res.glb");
			pub const LOW: AssetPath =
				AssetPath::new("vegetation/sticks/standard/trunk_001_low_res.glb");
		}
	}
}

/// Foliage GLBs under `vegetation/foliage/`.
pub mod foliage {
	use super::AssetPath;

	/// Kits under `vegetation/foliage/standard/`.
	pub mod standard {
		use super::AssetPath;

		pub const LAYERED_BALL_HIGH: AssetPath =
			AssetPath::new("vegetation/foliage/standard/layered_ball_001_high_res.glb");
		pub const LAYERED_BALL_MID: AssetPath =
			AssetPath::new("vegetation/foliage/standard/layered_ball_001_mid_res.glb");
		pub const LAYERED_BALL_LOW: AssetPath =
			AssetPath::new("vegetation/foliage/standard/layered_ball_001_low_res.glb");

		pub const CHEAP_BALL_HIGH: AssetPath =
			AssetPath::new("vegetation/foliage/standard/cheap_ball_001_high_res.glb");
		pub const CHEAP_BALL_MID: AssetPath =
			AssetPath::new("vegetation/foliage/standard/cheap_ball_001_mid_res.glb");
		pub const CHEAP_BALL_LOW: AssetPath =
			AssetPath::new("vegetation/foliage/standard/cheap_ball_001_low_res.glb");

		/// Point tip (degenerate length); prefer [`STRAIGHT_FROND_SEGMENT_*`] for strands.
		pub const STRAIGHT_FROND_HIGH: AssetPath =
			AssetPath::new("vegetation/foliage/standard/straight_frond_001_high_res.glb");
		pub const STRAIGHT_FROND_MID: AssetPath =
			AssetPath::new("vegetation/foliage/standard/straight_frond_001_mid_res.glb");
		pub const STRAIGHT_FROND_LOW: AssetPath =
			AssetPath::new("vegetation/foliage/standard/straight_frond_001_low_res.glb");

		/// Square-ended segment: \(Y \in [0, 1]\), \(X \in [-0.1, 0.1]\), \(Z\) negligible.
		pub const STRAIGHT_FROND_SEGMENT_HIGH: AssetPath =
			AssetPath::new("vegetation/foliage/standard/straight_frond_segment_001_high_res.glb");
		pub const STRAIGHT_FROND_SEGMENT_MID: AssetPath =
			AssetPath::new("vegetation/foliage/standard/straight_frond_segment_001_mid_res.glb");
		pub const STRAIGHT_FROND_SEGMENT_LOW: AssetPath =
			AssetPath::new("vegetation/foliage/standard/straight_frond_segment_001_low_res.glb");
	}
}
