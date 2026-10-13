//! Building-level structural LOD probe (meters outside XZ footprint).
//!
//! Distinct from mesh-resolution probes ([`crate::panels::PanelLodProbe`], …):
//! this selects which *layers* of authored IR a composite building emits
//! (e.g. internal walls on High only) and holds far-band [`MassingVolume`]s.
//!
//! Footprints are authored in the host's **local** XZ. Fine-phase updates map the
//! viewer into that local frame via [`GlobalTransform`] so gallery offsets work.

use bevy::prelude::{
	App, Component, GlobalTransform, IntoScheduleConfigs, Plugin, Query, Transform, Update, With,
};
use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodSceneLevel, LodSceneStatus};
use lod::LodRefreshSystems;

use crate::massing::{is_ultralow_massing, MassingVolume};

/// High while the viewer is at most this many meters outside the XZ perimeter.
pub const STRUCTURAL_HIGH_OUTSIDE_METERS: f32 = 80.0;

/// Medium (exterior kits) while the viewer is at most this many meters outside.
pub const STRUCTURAL_MEDIUM_OUTSIDE_METERS: f32 = 220.0;

/// Low (authored massing volumes) while the viewer is at most this many meters outside.
pub const STRUCTURAL_LOW_OUTSIDE_METERS: f32 = 520.0;

/// Viewer distance band for whole-building structural thinning.
///
/// Volumes are in the **host local** frame. Distance is planar meters outside
/// the nearest volume footprint (0 inside).
#[derive(Debug, Clone, PartialEq, Component)]
pub struct BuildingStructuralLodProbe {
	pub volumes: Vec<MassingVolume>,
	pub high_outside_meters: f32,
	pub medium_outside_meters: f32,
	pub low_outside_meters: f32,
}

impl Default for BuildingStructuralLodProbe {
	fn default() -> Self {
		Self {
			volumes: Vec::new(),
			high_outside_meters: STRUCTURAL_HIGH_OUTSIDE_METERS,
			medium_outside_meters: STRUCTURAL_MEDIUM_OUTSIDE_METERS,
			low_outside_meters: STRUCTURAL_LOW_OUTSIDE_METERS,
		}
	}
}

impl BuildingStructuralLodProbe {
	pub fn new(footprints: impl IntoIterator<Item = Aabb2d>) -> Self {
		Self::from_volumes(footprints.into_iter().map(|xz| MassingVolume::cuboid(xz, 0.0, 1.0)))
	}

	pub fn from_volumes(volumes: impl IntoIterator<Item = MassingVolume>) -> Self {
		Self { volumes: volumes.into_iter().collect(), ..Self::default() }
	}

	pub fn from_aabb3d_xz(min: Vec3, max: Vec3) -> Self {
		let xz = Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) };
		let y0 = min.y.min(max.y);
		let height = (max.y - min.y).abs().max(1e-3);
		Self::from_volumes([MassingVolume::cuboid(xz, y0, height)])
	}

	pub fn footprints(&self) -> Vec<Aabb2d> {
		self.volumes.iter().map(MassingVolume::aabb_xz).collect()
	}

	pub fn with_high_outside_meters(mut self, meters: f32) -> Self {
		self.high_outside_meters = meters.max(0.0);
		self
	}

	pub fn with_medium_outside_meters(mut self, meters: f32) -> Self {
		self.medium_outside_meters = meters.max(0.0);
		self
	}

	pub fn with_low_outside_meters(mut self, meters: f32) -> Self {
		self.low_outside_meters = meters.max(0.0);
		self
	}

	pub fn with_height(mut self, height: f32) -> Self {
		let height = height.max(1e-3);
		for volume in &mut self.volumes {
			volume.height = height;
		}
		self
	}

	pub fn with_y0(mut self, y0: f32) -> Self {
		for volume in &mut self.volumes {
			volume.y0 = y0;
		}
		self
	}

	/// High / Medium / Low far edges, with Medium ≥ High and Low ≥ Medium.
	pub fn band_meters(&self) -> (f32, f32, f32) {
		let high = self.high_outside_meters.max(0.0);
		let medium = self.medium_outside_meters.max(high);
		let low = self.low_outside_meters.max(medium);
		(high, medium, low)
	}

	/// Append another probe's volumes (keep the tighter cutoffs).
	pub fn merge(mut self, other: Self) -> Self {
		self.volumes.extend(other.volumes);
		self.high_outside_meters = self.high_outside_meters.min(other.high_outside_meters);
		self.medium_outside_meters = self.medium_outside_meters.min(other.medium_outside_meters);
		self.low_outside_meters = self.low_outside_meters.min(other.low_outside_meters);
		self
	}

	/// Meters outside the nearest volume in local XZ (0 when inside any).
	pub fn distance_outside_local(&self, viewer_local: Vec3) -> f32 {
		distance_outside_footprints(viewer_local, &self.footprints())
	}

	/// [`distance_outside_local`] treating `viewer.translation` as already local.
	pub fn distance_outside(&self, viewer: &Transform) -> f32 {
		self.distance_outside_local(viewer.translation)
	}

	pub fn level_for_local(&self, viewer_local: Vec3) -> LodSceneLevel {
		let d = self.distance_outside_local(viewer_local);
		let (high, medium, low) = self.band_meters();
		if d <= high {
			LodSceneLevel::High
		} else if d <= medium {
			LodSceneLevel::Medium
		} else if d <= low {
			LodSceneLevel::Low
		} else {
			LodSceneLevel::UltraLow
		}
	}

	/// Level when `viewer` is in the same space as the footprints (usually local).
	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		self.level_for_local(viewer.translation)
	}

	/// Level for a world-space viewer against local footprints on `host_global`.
	pub fn level_for_world(
		&self,
		viewer_world: Vec3,
		host_global: &GlobalTransform,
	) -> LodSceneLevel {
		self.level_for_local(self.viewer_local(viewer_world, host_global))
	}

	/// [`distance_outside_local`] after mapping `viewer_world` through `host_global`.
	pub fn distance_outside_world(&self, viewer_world: Vec3, host_global: &GlobalTransform) -> f32 {
		self.distance_outside_local(self.viewer_local(viewer_world, host_global))
	}

	pub fn status_for_world(
		&self,
		previous_world: Vec3,
		current_world: Vec3,
		host_global: &GlobalTransform,
	) -> LodSceneStatus {
		let prev = self.level_for_world(previous_world, host_global);
		let curr = self.level_for_world(current_world, host_global);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	pub fn status_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.level_for(lod_ref.previous_transform);
		let curr = self.level_for(lod_ref.current_transform);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	fn viewer_local(&self, viewer_world: Vec3, host_global: &GlobalTransform) -> Vec3 {
		host_global.affine().inverse().transform_point3(viewer_world)
	}

	/// Coarse local AABB covering all volumes.
	pub fn footprint_aabb(&self) -> Aabb3d {
		let Some(xz) = self.footprint_xz() else {
			return Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		};
		let mut y0 = f32::INFINITY;
		let mut y1 = f32::NEG_INFINITY;
		for volume in &self.volumes {
			y0 = y0.min(volume.y0);
			y1 = y1.max(volume.y0 + volume.height);
			if let Some(roof) = &volume.roof {
				y1 = y1.max(volume.y0 + volume.height + roof.rise);
			}
		}
		if !y0.is_finite() {
			y0 = 0.0;
			y1 = 1.0;
		}
		Aabb3d::from_min_max(
			Vec3::new(xz.min.x, y0, xz.min.y),
			Vec3::new(xz.max.x, y1.max(y0 + 1e-3), xz.max.y),
		)
	}

	/// Union of authored XZ footprints (`Aabb2d.y` = world \(z\)).
	pub fn footprint_xz(&self) -> Option<Aabb2d> {
		if self.volumes.is_empty() {
			return None;
		}
		let mut min_x = f32::INFINITY;
		let mut max_x = f32::NEG_INFINITY;
		let mut min_z = f32::INFINITY;
		let mut max_z = f32::NEG_INFINITY;
		for volume in &self.volumes {
			let rect = volume.aabb_xz();
			min_x = min_x.min(rect.min.x);
			max_x = max_x.max(rect.max.x);
			min_z = min_z.min(rect.min.y);
			max_z = max_z.max(rect.max.y);
		}
		Some(Aabb2d { min: Vec2::new(min_x, min_z), max: Vec2::new(max_x, max_z) })
	}

	/// Volumes drawn at `level` (UltraLow keeps structural primitives, drops roofs).
	pub fn massing_volumes(&self, level: LodSceneLevel) -> Vec<MassingVolume> {
		let ultralow = is_ultralow_massing(level);
		self.volumes.iter().flat_map(|volume| volume.drawn_at(ultralow)).collect()
	}
}

/// Planar distance outside an XZ footprint (`Aabb2d.y` = world \(z\)).
pub fn distance_outside_aabb2d_xz(p: Vec3, rect: &Aabb2d) -> f32 {
	let dx = if p.x < rect.min.x {
		rect.min.x - p.x
	} else if p.x > rect.max.x {
		p.x - rect.max.x
	} else {
		0.0
	};
	let dz = if p.z < rect.min.y {
		rect.min.y - p.z
	} else if p.z > rect.max.y {
		p.z - rect.max.y
	} else {
		0.0
	};
	if dx <= 0.0 && dz <= 0.0 {
		0.0
	} else {
		(dx * dx + dz * dz).sqrt()
	}
}

/// Min distance outside a set of footprints (`∞` when empty).
pub fn distance_outside_footprints(p: Vec3, footprints: &[Aabb2d]) -> f32 {
	if footprints.is_empty() {
		return f32::INFINITY;
	}
	footprints
		.iter()
		.map(|r| distance_outside_aabb2d_xz(p, r))
		.fold(f32::INFINITY, f32::min)
}

/// Update structural building host levels from the [`lod::LodViewer`] pose.
///
/// Viewer is world-space; footprints are host-local — convert via [`GlobalTransform`].
pub fn update_building_structural_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<
		(&BuildingStructuralLodProbe, &GlobalTransform, &mut LodSceneLevel),
		With<LodSceneHost>,
	>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	let viewer_world = pose.current.translation;
	for (probe, global, mut level) in &mut hosts {
		let next = probe.level_for_world(viewer_world, global);
		if *level != next {
			*level = next;
		}
	}
}

/// Live [`GlobalTransform`] correction for structural building hosts.
///
/// Produce / [`crate::ComponentsOnly::scene_lod_level`] use the pose stored on
/// the host. This system remaps through the composed world transform so a
/// parent offset (gallery, nested `Built` column) still upgrades.
pub struct BuildingStructuralLodPlugin;

impl Plugin for BuildingStructuralLodPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			update_building_structural_host_levels.in_set(LodRefreshSystems::UpdateLevels),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::massing::MassingKind;

	#[test]
	fn inside_footprint_is_high() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		);
		let viewer = Transform::from_xyz(0.0, 1.5, 0.0);
		assert_eq!(probe.distance_outside(&viewer), 0.0);
		assert_eq!(probe.level_for(&viewer), LodSceneLevel::High);
	}

	#[test]
	fn switches_to_medium_past_high_outside_meters() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		)
		.with_high_outside_meters(20.0)
		.with_medium_outside_meters(40.0)
		.with_low_outside_meters(80.0);
		let near = Transform::from_xyz(5.0 + 19.0, 1.5, 0.0);
		let far = Transform::from_xyz(5.0 + 21.0, 1.5, 0.0);
		assert_eq!(probe.level_for(&near), LodSceneLevel::High);
		assert_eq!(probe.level_for(&far), LodSceneLevel::Medium);
	}

	#[test]
	fn switches_to_low_and_ultralow() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		)
		.with_high_outside_meters(20.0)
		.with_medium_outside_meters(40.0)
		.with_low_outside_meters(80.0);
		let low = Transform::from_xyz(5.0 + 41.0, 1.5, 0.0);
		let ultra = Transform::from_xyz(5.0 + 81.0, 1.5, 0.0);
		assert_eq!(probe.level_for(&low), LodSceneLevel::Low);
		assert_eq!(probe.level_for(&ultra), LodSceneLevel::UltraLow);
		assert_eq!(probe.massing_volumes(LodSceneLevel::Low).len(), 1);
		assert_eq!(probe.massing_volumes(LodSceneLevel::UltraLow).len(), 1);
	}

	#[test]
	fn ultralow_keeps_each_structural_volume() {
		let a = MassingVolume::cuboid(
			Aabb2d { min: Vec2::new(-10.0, -2.0), max: Vec2::new(-6.0, 2.0) },
			0.0,
			3.0,
		);
		let b = MassingVolume::cylinder(Vec2::new(8.0, 0.0), 2.0, 0.0, 8.0);
		let roof = MassingVolume::roof_pitch(
			Aabb2d { min: Vec2::new(-10.0, -2.0), max: Vec2::new(-6.0, 2.0) },
			3.0,
			1.5,
			true,
		);
		let probe = BuildingStructuralLodProbe::from_volumes([a, b, roof]);
		assert_eq!(probe.massing_volumes(LodSceneLevel::Low).len(), 3);
		let ultra = probe.massing_volumes(LodSceneLevel::UltraLow);
		assert_eq!(ultra.len(), 2);
		assert!(ultra.iter().all(|v| !matches!(v.kind, MassingKind::RoofPitch { .. })));
	}

	#[test]
	fn merge_uses_nearest_footprint() {
		let a = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-10.0, 0.0, -2.0),
			Vec3::new(-6.0, 3.0, 2.0),
		);
		let b = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(6.0, 0.0, -2.0),
			Vec3::new(10.0, 3.0, 2.0),
		);
		let probe = a.merge(b);
		let near_b = Transform::from_xyz(10.0 + 5.0, 1.5, 0.0);
		assert!((probe.distance_outside(&near_b) - 5.0).abs() < 1e-4);
	}

	#[test]
	fn status_for_world_tracks_band_changes() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		)
		.with_high_outside_meters(20.0)
		.with_medium_outside_meters(40.0)
		.with_low_outside_meters(80.0);
		let host = GlobalTransform::from_translation(Vec3::new(200.0, 0.0, 0.0));
		assert_eq!(
			probe.status_for_world(
				Vec3::new(200.0 + 5.0 + 10.0, 1.5, 0.0),
				Vec3::new(200.0 + 5.0 + 10.0, 1.5, 0.0),
				&host
			),
			LodSceneStatus::Unchanged
		);
		assert_eq!(
			probe.status_for_world(
				Vec3::new(200.0 + 5.0 + 10.0, 1.5, 0.0),
				Vec3::new(200.0 + 5.0 + 21.0, 1.5, 0.0),
				&host
			),
			LodSceneStatus::Changed(LodSceneLevel::Medium)
		);
	}

	#[test]
	fn world_viewer_respects_host_translation() {
		let probe = BuildingStructuralLodProbe::from_aabb3d_xz(
			Vec3::new(-5.0, 0.0, -5.0),
			Vec3::new(5.0, 3.0, 5.0),
		)
		.with_high_outside_meters(20.0);
		let host = GlobalTransform::from_translation(Vec3::new(200.0, 0.0, 0.0));
		let near_world = Vec3::new(200.0 + 5.0 + 10.0, 1.5, 0.0);
		assert_eq!(probe.level_for_world(near_world, &host), LodSceneLevel::High);
		assert_eq!(
			probe.level_for(&Transform::from_translation(near_world)),
			LodSceneLevel::Medium
		);
	}
}
