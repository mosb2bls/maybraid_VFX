//! Horizontal elevation wedge placeholder. Unused — polyline joinery uses joints only.

/// Elevation kink filler (not allocated by polyline tessellation).
#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStoneworkWedge;

crate::impl_empty_lod_scene!(RoughStoneworkWedge);
