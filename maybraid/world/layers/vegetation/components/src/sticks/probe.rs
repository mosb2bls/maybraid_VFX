//! Distance / extent probe for stick LOD hosts.

use bevy::prelude::{Component, Query, Transform, With};
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodSceneLevel, LodSceneStatus};

use crate::lod_band::{characteristic_extent_abs, placement_center, DistanceLodBand};
use crate::placed::Placement;
use crate::sticks::geometry::StickGeometry;

/// `distance / radius` (or trunk extent) at or below this → High stick mesh.
pub const STICK_HIGH_FACTOR: f32 = 10.0;
/// `distance / radius` (or trunk extent) at or below this → Medium stick mesh.
pub const STICK_MEDIUM_FACTOR: f32 = 50.0;
/// `distance / radius` (or trunk extent) at or below this → Low stick mesh; beyond → UltraLow (empty).
pub const STICK_LOW_FACTOR: f32 = 200.0;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct StickLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl StickLodProbe {
	/// Extent is **radius/girth** for segments; trunks keep the max-axis characteristic extent
	/// (typically length-dominated).
	pub fn from_stick(placement: &Placement, geometry: StickGeometry) -> Self {
		let extent = match geometry {
			StickGeometry::Trunk => characteristic_extent_abs(placement),
			StickGeometry::Segment => {
				placement.scale.x.abs().max(placement.scale.z.abs()).max(1e-4)
			}
		};
		Self { center: placement_center(placement), extent }
	}

	fn band_to_level(band: DistanceLodBand) -> LodSceneLevel {
		match band {
			DistanceLodBand::High => LodSceneLevel::High,
			DistanceLodBand::Medium => LodSceneLevel::Medium,
			DistanceLodBand::Low => LodSceneLevel::Low,
			// Stick mesh UltraLow is a real empty tier (not collapsed onto Low).
			DistanceLodBand::UltraLow => LodSceneLevel::UltraLow,
		}
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		let distance = viewer.translation.distance(self.center);
		let factor = distance / self.extent.max(1e-4);
		Self::band_to_level(DistanceLodBand::from_factors(
			factor,
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		))
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = Self::band_to_level(DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center) / self.extent.max(1e-4),
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		));
		let curr = Self::band_to_level(DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center) / self.extent.max(1e-4),
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		));
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}
}

/// Update stick host levels from the [`lod::LodViewer`] pose.
pub fn update_stick_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&StickLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	let viewer = pose.current;
	for (probe, mut level) in &mut hosts {
		let next = probe.level_for(&viewer);
		if *level != next {
			*level = next;
		}
	}
}
