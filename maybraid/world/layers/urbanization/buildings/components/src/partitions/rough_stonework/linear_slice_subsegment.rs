//! Slice-height linear rough stonework subsegment.

/// Short vertical slice segment for door/window frames on straight walls.
#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStoneworkLinearSliceSubsegment;

crate::impl_empty_lod_scene!(RoughStoneworkLinearSliceSubsegment);
