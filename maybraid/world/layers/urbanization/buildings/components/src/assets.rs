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

	/// GLTF scene label for [`bevy::scene::WorldAssetRoot`] (`path#Scene0`).
	pub fn gltf_scene_0(self) -> String {
		format!("{}#Scene0", self.0)
	}

	/// Shared [`scene_ref::SceneRef`] for this GLB (scene 0).
	pub fn scene_ref(self) -> scene_ref::SceneRef {
		scene_ref::SceneRef::glb(self.0)
	}
}

impl std::fmt::Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

/// Shared urban panel GLBs under `urban/panels/` (rectangles, triangles, fillers).
pub mod panels {
	use super::AssetPath;

	/// Canonical unit right-triangle primitive (no style / LOD variants).
	pub const UNIT_RIGHT_TRIANGLE: AssetPath =
		AssetPath::new("urban/panels/unit_right_triangle.glb");

	/// Rectangle + right-triangle LOD triad under `urban/panels/<dir>/`.
	macro_rules! panel_style_kit {
		($mod:ident, $dir:literal) => {
			pub mod $mod {
				use super::AssetPath;

				pub const RECTANGLE_HIGH: AssetPath =
					AssetPath::new(concat!("urban/panels/", $dir, "/rectangle_001_high_res.glb"));
				pub const RECTANGLE_MID: AssetPath =
					AssetPath::new(concat!("urban/panels/", $dir, "/rectangle_001_mid_res.glb"));
				pub const RECTANGLE_LOW: AssetPath =
					AssetPath::new(concat!("urban/panels/", $dir, "/rectangle_001_low_res.glb"));

				/// Unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
				pub const RIGHT_TRIANGLE_HIGH: AssetPath = AssetPath::new(concat!(
					"urban/panels/",
					$dir,
					"/right_triangle_001_high_res.glb"
				));
				pub const RIGHT_TRIANGLE_MID: AssetPath = AssetPath::new(concat!(
					"urban/panels/",
					$dir,
					"/right_triangle_001_mid_res.glb"
				));
				pub const RIGHT_TRIANGLE_LOW: AssetPath = AssetPath::new(concat!(
					"urban/panels/",
					$dir,
					"/right_triangle_001_low_res.glb"
				));
			}
		};
	}

	panel_style_kit!(default, "default");
	panel_style_kit!(desert_web, "desert_web");
	panel_style_kit!(flat, "flat");
	panel_style_kit!(myrs_ornate, "myrs_ornate");
	panel_style_kit!(rib_and_plank, "rib_and_plank");
	panel_style_kit!(shepherds_thatch, "shepherds_thatch");
	panel_style_kit!(tent_angles, "tent_angles");
	panel_style_kit!(terracotta_tubes, "terracotta_tubes");

	pub mod rough_stonework {
		use super::AssetPath;

		/// Default-resolution rectangle when a single path is needed (high-res LOD).
		pub const RECTANGLE: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/rectangle_001_high_res.glb");
		/// Resolution triad for the unit rectangle panel (\(X, Z \in [0, 1]\)).
		pub const RECTANGLE_HIGH: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/rectangle_001_high_res.glb");
		pub const RECTANGLE_MID: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/rectangle_001_mid_res.glb");
		pub const RECTANGLE_LOW: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/rectangle_001_low_res.glb");

		/// Circle−inscribed-square filler (four yaws fill a circular ring).
		pub const INSCRIBED_SQUARE: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/inscribed_square_001.glb");

		/// Unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
		pub const RIGHT_TRIANGLE_HIGH: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/right_triangle_001_high_res.glb");
		pub const RIGHT_TRIANGLE_MID: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/right_triangle_001_mid_res.glb");
		pub const RIGHT_TRIANGLE_LOW: AssetPath =
			AssetPath::new("urban/panels/rough_stonework/right_triangle_001_low_res.glb");
	}
}

/// Urban joint GLBs under `urban/joints/`.
pub mod joints {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// Circular joint between partition segments (\(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\)).
		/// Low / ultra-low LOD omit this filler (no low-res asset).
		pub const JOINT_HIGH: AssetPath =
			AssetPath::new("urban/joints/rough_stonework/joint_001_high_res.glb");
		pub const JOINT_MID: AssetPath =
			AssetPath::new("urban/joints/rough_stonework/joint_001_mid_res.glb");
	}
}

/// Urban arc GLBs under `urban/arcs/` (bodies, slices, frames).
pub mod arcs {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// 180° arc rough stonework.
		pub const ARC_180: AssetPath = AssetPath::new("urban/arcs/rough_stonework/arc_180_001.glb");
		pub const ARC_180_HIGH: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_180_001_high_res.glb");
		pub const ARC_180_MID: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_180_001_mid_res.glb");
		pub const ARC_180_LOW: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_180_001_low_res.glb");

		/// 90° arc rough stonework.
		pub const ARC_90: AssetPath = AssetPath::new("urban/arcs/rough_stonework/arc_90_001.glb");
		pub const ARC_90_HIGH: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_001_high_res.glb");
		pub const ARC_90_MID: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_001_mid_res.glb");
		pub const ARC_90_LOW: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_001_low_res.glb");

		/// 15° arc rough stonework.
		pub const ARC_15: AssetPath = AssetPath::new("urban/arcs/rough_stonework/arc_15_001.glb");
		pub const ARC_15_HIGH: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_001_high_res.glb");
		pub const ARC_15_MID: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_001_mid_res.glb");
		pub const ARC_15_LOW: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_001_low_res.glb");

		/// 90° slice rough stonework.
		pub const SLICE_90: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_slice_001.glb");
		pub const SLICE_90_HIGH: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_slice_001_high_res.glb");
		pub const SLICE_90_MID: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_slice_001_mid_res.glb");
		pub const SLICE_90_LOW: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_90_slice_001_low_res.glb");

		/// 15° slice rough stonework.
		pub const SLICE_15: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_slice_001.glb");
		pub const SLICE_15_HIGH: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_slice_001_high_res.glb");
		pub const SLICE_15_MID: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_slice_001_mid_res.glb");
		pub const SLICE_15_LOW: AssetPath =
			AssetPath::new("urban/arcs/rough_stonework/arc_15_slice_001_low_res.glb");
	}
}

/// Urban partition GLBs under `urban/partitions/`.
pub mod partitions {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;
		use crate::assets::{arcs, panels};

		/// Straight / linear rough stonework segment (shared panel rectangle).
		pub const LINEAR_HIGH: AssetPath = panels::rough_stonework::RECTANGLE_HIGH;
		pub const LINEAR_MID: AssetPath = panels::rough_stonework::RECTANGLE_MID;
		pub const LINEAR_LOW: AssetPath = panels::rough_stonework::RECTANGLE_LOW;

		/// Arc bodies / slices (shared kits under `urban/arcs/`).
		pub const ARC_180: AssetPath = arcs::rough_stonework::ARC_180;
		pub const ARC_180_HIGH: AssetPath = arcs::rough_stonework::ARC_180_HIGH;
		pub const ARC_180_MID: AssetPath = arcs::rough_stonework::ARC_180_MID;
		pub const ARC_180_LOW: AssetPath = arcs::rough_stonework::ARC_180_LOW;

		pub const ARC_90: AssetPath = arcs::rough_stonework::ARC_90;
		pub const ARC_90_HIGH: AssetPath = arcs::rough_stonework::ARC_90_HIGH;
		pub const ARC_90_MID: AssetPath = arcs::rough_stonework::ARC_90_MID;
		pub const ARC_90_LOW: AssetPath = arcs::rough_stonework::ARC_90_LOW;

		pub const ARC_15: AssetPath = arcs::rough_stonework::ARC_15;
		pub const ARC_15_HIGH: AssetPath = arcs::rough_stonework::ARC_15_HIGH;
		pub const ARC_15_MID: AssetPath = arcs::rough_stonework::ARC_15_MID;
		pub const ARC_15_LOW: AssetPath = arcs::rough_stonework::ARC_15_LOW;

		pub const SLICE_90: AssetPath = arcs::rough_stonework::SLICE_90;
		pub const SLICE_90_HIGH: AssetPath = arcs::rough_stonework::SLICE_90_HIGH;
		pub const SLICE_90_MID: AssetPath = arcs::rough_stonework::SLICE_90_MID;
		pub const SLICE_90_LOW: AssetPath = arcs::rough_stonework::SLICE_90_LOW;

		pub const SLICE_15: AssetPath = arcs::rough_stonework::SLICE_15;
		pub const SLICE_15_HIGH: AssetPath = arcs::rough_stonework::SLICE_15_HIGH;
		pub const SLICE_15_MID: AssetPath = arcs::rough_stonework::SLICE_15_MID;
		pub const SLICE_15_LOW: AssetPath = arcs::rough_stonework::SLICE_15_LOW;

		/// Circular joint (shared kit under `urban/joints/`).
		pub const JOINT_HIGH: AssetPath = crate::assets::joints::rough_stonework::JOINT_HIGH;
		pub const JOINT_MID: AssetPath = crate::assets::joints::rough_stonework::JOINT_MID;
	}
}

/// Urban floor GLBs under `urban/floors/`.
pub mod floors {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;
		use crate::assets::panels;

		/// Unit rectangular floor slab.
		pub const RECTANGLE: AssetPath =
			AssetPath::new("urban/floors/rough_stonework/rough_stonework_001.glb");
		/// Circle−inscribed-square floor cap (shared panel kit).
		pub const CIRCLE_INSCRIBED_SQUARE: AssetPath = panels::rough_stonework::INSCRIBED_SQUARE;
	}
}

/// Urban stair GLBs under `urban/stairs/`.
pub mod stairs {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// Unit tread cube (\(X = Y = Z \in [-1, 1]\), left face −Z).
		pub const TREAD: AssetPath =
			AssetPath::new("urban/stairs/rough_stonework/rough_stonework_tread_001.glb");
	}
}

/// Urban roof GLBs — triangle kits live under `urban/panels/`.
pub mod roofs {
	use super::AssetPath;
	use crate::assets::panels;

	/// Canonical unit right-triangle primitive (no style / LOD variants).
	pub const UNIT_RIGHT_TRIANGLE: AssetPath = panels::UNIT_RIGHT_TRIANGLE;

	pub mod shepherds_thatch {
		use super::AssetPath;
		use crate::assets::panels;

		pub const RIGHT_TRIANGLE_HIGH: AssetPath = panels::shepherds_thatch::RIGHT_TRIANGLE_HIGH;
		pub const RIGHT_TRIANGLE_MID: AssetPath = panels::shepherds_thatch::RIGHT_TRIANGLE_MID;
		pub const RIGHT_TRIANGLE_LOW: AssetPath = panels::shepherds_thatch::RIGHT_TRIANGLE_LOW;
	}
}
