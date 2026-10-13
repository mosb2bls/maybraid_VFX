//! Panel material style.

use crate::assets::panels::{
	default as panel_default, desert_web, flat, myrs_ornate, rib_and_plank, rough_stonework,
	shepherds_thatch, tent_angles, terracotta_tubes,
};
use crate::assets::AssetPath;
use crate::panels::geometry::PanelKitCaps;

/// Material look for shared panel geometry ([`crate::panels::PanelNode`]).
///
/// Variant names match `urban/panels/<snake_case>/` kit folders from art/assets
/// (see [PR #568](https://github.com/ramate-io/maybraid/pull/568)).
/// High / mid / low use each style's triad; UltraLow always uses the shared flat
/// low-res kit ([`crate::panels::PANEL_ULTRA_LOW_RECTANGLE`] /
/// [`crate::panels::PANEL_ULTRA_LOW_RIGHT_TRIANGLE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PanelStyle {
	#[default]
	RoughStonework,
	ShepherdsThatch,
	/// Neutral / generic panel kit (`urban/panels/default/`).
	Default,
	DesertWeb,
	Flat,
	MyrsOrnate,
	RibAndPlank,
	TentAngles,
	TerracottaTubes,
}

impl PanelStyle {
	/// Kit capabilities used by [`crate::panels::PanelGeometry::flatten`].
	pub fn kit_caps(self) -> PanelKitCaps {
		PanelKitCaps::from(self)
	}

	/// High / mid / low rectangle kit paths, when this style has a rectangle body.
	pub fn rectangle_lod(self) -> Option<(AssetPath, AssetPath, AssetPath)> {
		Some(match self {
			Self::RoughStonework => (
				rough_stonework::RECTANGLE_HIGH,
				rough_stonework::RECTANGLE_MID,
				rough_stonework::RECTANGLE_LOW,
			),
			Self::ShepherdsThatch => (
				shepherds_thatch::RECTANGLE_HIGH,
				shepherds_thatch::RECTANGLE_MID,
				shepherds_thatch::RECTANGLE_LOW,
			),
			Self::Default => (
				panel_default::RECTANGLE_HIGH,
				panel_default::RECTANGLE_MID,
				panel_default::RECTANGLE_LOW,
			),
			Self::DesertWeb => {
				(desert_web::RECTANGLE_HIGH, desert_web::RECTANGLE_MID, desert_web::RECTANGLE_LOW)
			}
			Self::Flat => (flat::RECTANGLE_HIGH, flat::RECTANGLE_MID, flat::RECTANGLE_LOW),
			Self::MyrsOrnate => (
				myrs_ornate::RECTANGLE_HIGH,
				myrs_ornate::RECTANGLE_MID,
				myrs_ornate::RECTANGLE_LOW,
			),
			Self::RibAndPlank => (
				rib_and_plank::RECTANGLE_HIGH,
				rib_and_plank::RECTANGLE_MID,
				rib_and_plank::RECTANGLE_LOW,
			),
			Self::TentAngles => (
				tent_angles::RECTANGLE_HIGH,
				tent_angles::RECTANGLE_MID,
				tent_angles::RECTANGLE_LOW,
			),
			Self::TerracottaTubes => (
				terracotta_tubes::RECTANGLE_HIGH,
				terracotta_tubes::RECTANGLE_MID,
				terracotta_tubes::RECTANGLE_LOW,
			),
		})
	}

	/// High / mid / low right-triangle kit paths.
	pub fn right_triangle_lod(self) -> Option<(AssetPath, AssetPath, AssetPath)> {
		Some(match self {
			Self::RoughStonework => (
				rough_stonework::RIGHT_TRIANGLE_HIGH,
				rough_stonework::RIGHT_TRIANGLE_MID,
				rough_stonework::RIGHT_TRIANGLE_LOW,
			),
			Self::ShepherdsThatch => (
				shepherds_thatch::RIGHT_TRIANGLE_HIGH,
				shepherds_thatch::RIGHT_TRIANGLE_MID,
				shepherds_thatch::RIGHT_TRIANGLE_LOW,
			),
			Self::Default => (
				panel_default::RIGHT_TRIANGLE_HIGH,
				panel_default::RIGHT_TRIANGLE_MID,
				panel_default::RIGHT_TRIANGLE_LOW,
			),
			Self::DesertWeb => (
				desert_web::RIGHT_TRIANGLE_HIGH,
				desert_web::RIGHT_TRIANGLE_MID,
				desert_web::RIGHT_TRIANGLE_LOW,
			),
			Self::Flat => {
				(flat::RIGHT_TRIANGLE_HIGH, flat::RIGHT_TRIANGLE_MID, flat::RIGHT_TRIANGLE_LOW)
			}
			Self::MyrsOrnate => (
				myrs_ornate::RIGHT_TRIANGLE_HIGH,
				myrs_ornate::RIGHT_TRIANGLE_MID,
				myrs_ornate::RIGHT_TRIANGLE_LOW,
			),
			Self::RibAndPlank => (
				rib_and_plank::RIGHT_TRIANGLE_HIGH,
				rib_and_plank::RIGHT_TRIANGLE_MID,
				rib_and_plank::RIGHT_TRIANGLE_LOW,
			),
			Self::TentAngles => (
				tent_angles::RIGHT_TRIANGLE_HIGH,
				tent_angles::RIGHT_TRIANGLE_MID,
				tent_angles::RIGHT_TRIANGLE_LOW,
			),
			Self::TerracottaTubes => (
				terracotta_tubes::RIGHT_TRIANGLE_HIGH,
				terracotta_tubes::RIGHT_TRIANGLE_MID,
				terracotta_tubes::RIGHT_TRIANGLE_LOW,
			),
		})
	}
}

impl From<PanelStyle> for PanelKitCaps {
	fn from(_style: PanelStyle) -> Self {
		// All current panel kits ship rectangle + right-triangle LOD triads.
		Self::WITH_RECTANGLE
	}
}
