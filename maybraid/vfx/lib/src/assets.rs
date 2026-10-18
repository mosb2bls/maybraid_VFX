//! Flipbook metadata and compiled GPU handles.

use bevy::prelude::*;

use crate::atlas::{fire_cell, paint_atlas, paint_spark_taper, smoke_cell};

/// Texture handle, atlas grid, frame count, and playback duration.
#[derive(Clone, Debug)]
pub struct FlipbookAsset {
	pub image: Handle<Image>,
	pub grid: UVec2,
	pub frame_count: u32,
	pub playback: f32,
}

impl FlipbookAsset {
	pub fn last_frame(&self) -> u32 {
		self.frame_count.saturating_sub(1)
	}
}

pub const FIRE_GRID: UVec2 = UVec2::new(8, 8);
pub const SMOKE_GRID: UVec2 = UVec2::new(8, 8);
pub const ATLAS_CELL: u32 = 64;
pub const FIRE_PLAYBACK: f32 = 0.55;
pub const SMOKE_PLAYBACK: f32 = 2.4;

/// Flipbooks compiled for the first explosion set.
#[derive(Clone, Debug)]
pub struct VfxFlipbooks {
	pub fire: FlipbookAsset,
	pub smoke: FlipbookAsset,
	pub spark: Handle<Image>,
}

pub fn fire_flipbook(images: &mut Assets<Image>) -> FlipbookAsset {
	FlipbookAsset {
		image: images.add(paint_atlas(FIRE_GRID, ATLAS_CELL, fire_cell)),
		grid: FIRE_GRID,
		frame_count: FIRE_GRID.x * FIRE_GRID.y,
		playback: FIRE_PLAYBACK,
	}
}

pub fn smoke_flipbook(images: &mut Assets<Image>) -> FlipbookAsset {
	FlipbookAsset {
		image: images.add(paint_atlas(SMOKE_GRID, ATLAS_CELL, smoke_cell)),
		grid: SMOKE_GRID,
		frame_count: SMOKE_GRID.x * SMOKE_GRID.y,
		playback: SMOKE_PLAYBACK,
	}
}

pub fn spark_taper(images: &mut Assets<Image>) -> Handle<Image> {
	images.add(paint_spark_taper())
}
