//! 400 m mob cells and placement helpers.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use urbanization_cells::UrbanizationKind;

use crate::generation::{GroupKind, MobGroup, MobPlantHost};

pub const MOB_CELL_EXTENT: f32 = 400.0;
pub const MOB_WORLD_SEED: u64 = 42;
pub const MOB_CELL_OCCUPANCY_PERCENT: u64 = 35;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MobCellExtent {
	min: Vec3,
	max: Vec3,
}

impl MobCellExtent {
	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let half = MOB_CELL_EXTENT * 0.5;
		Self {
			min: Vec3::new(
				ix as f32 * MOB_CELL_EXTENT - half,
				0.0,
				iz as f32 * MOB_CELL_EXTENT - half,
			),
			max: Vec3::new(
				ix as f32 * MOB_CELL_EXTENT + half,
				0.0,
				iz as f32 * MOB_CELL_EXTENT + half,
			),
		}
	}

	/// Scheme-placed cell. Grid generate still uses [`Self::from_cell_index`].
	pub fn from_bounds(min: Vec3, max: Vec3) -> Self {
		Self { min, max }
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		let width = bounds.max.x - bounds.min.x;
		let depth = bounds.max.z - bounds.min.z;
		if (width - MOB_CELL_EXTENT).abs() > 1e-3 || (depth - MOB_CELL_EXTENT).abs() > 1e-3 {
			return None;
		}
		Some(Self { min: bounds.min.into(), max: bounds.max.into() })
	}

	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let min = Self::cell_index_containing(Vec3::new(region.min.x, 0.0, region.min.z));
		let max = Self::cell_index_containing(Vec3::new(
			(region.max.x - 1e-3).max(region.min.x),
			0.0,
			(region.max.z - 1e-3).max(region.min.z),
		));
		(min.0.min(max.0)..=min.0.max(max.0))
			.flat_map(|ix| {
				(min.1.min(max.1)..=min.1.max(max.1)).map(move |iz| Self::from_cell_index(ix, iz))
			})
			.collect()
	}

	/// [`OriginalId`]s of cells whose footprints overlap `region` on XZ.
	pub fn original_ids_overlapping(region: Aabb3d) -> Vec<OriginalId> {
		Self::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	/// Like [`Self::original_ids_overlapping`], but only cells that hold a mob group.
	pub fn occupied_original_ids_overlapping(region: Aabb3d) -> Vec<OriginalId> {
		Self::cells_overlapping(region)
			.into_iter()
			.filter(|extent| extent.group_seed().is_some())
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let half = MOB_CELL_EXTENT * 0.5;
		(
			((position.x + half) / MOB_CELL_EXTENT).floor() as i32,
			((position.z + half) / MOB_CELL_EXTENT).floor() as i32,
		)
	}

	pub fn center(self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	pub fn index(self) -> (i32, i32) {
		Self::cell_index_containing(self.center())
	}

	/// The seed of the one group this cell holds; `None` for an empty cell.
	pub fn group_seed(self) -> Option<u64> {
		let (ix, iz) = self.index();
		let seed = cell_seed(ix, iz);
		let occupied = (ix == 0 && iz == 0)
			|| mixed(seed ^ 0x6d6f_622d_6365_6c6c) % 100 < MOB_CELL_OCCUPANCY_PERCENT;
		occupied.then_some(seed)
	}
}

pub(crate) fn urbanization_weight(kind: UrbanizationKind) -> f32 {
	match kind {
		UrbanizationKind::None => 0.0,
		UrbanizationKind::RuralLife => 0.2,
		UrbanizationKind::Frontier => 0.4,
		UrbanizationKind::Townships => 0.55,
		UrbanizationKind::Colony => 0.7,
		UrbanizationKind::MixedAgeCity => 0.85,
		UrbanizationKind::ModernCity => 1.0,
	}
}

pub(crate) fn group_kind(kind: UrbanizationKind, seed: u64) -> GroupKind {
	match kind {
		UrbanizationKind::None => GroupKind::Wild,
		UrbanizationKind::RuralLife => GroupKind::Peaceful,
		UrbanizationKind::Townships | UrbanizationKind::Frontier => GroupKind::Frontier,
		UrbanizationKind::Colony => GroupKind::Warfront,
		UrbanizationKind::MixedAgeCity => {
			if mixed(seed) & 1 == 0 {
				GroupKind::Warfront
			} else {
				GroupKind::Dystopian
			}
		}
		UrbanizationKind::ModernCity => GroupKind::Dystopian,
	}
}

/// The hosts a group of `extent` around `origin` can reach.
pub(crate) fn hosts_near(hosts: &[MobPlantHost], origin: Vec2, extent: f32) -> Vec<MobPlantHost> {
	let half = extent * 0.5;
	hosts
		.iter()
		.copied()
		.filter(|host| {
			(host.xz.x - origin.x).abs() <= half + host.arrival_radius
				&& (host.xz.y - origin.y).abs() <= half + host.arrival_radius
		})
		.collect()
}

#[derive(Clone, Debug)]
pub struct MobCell {
	pub extent: MobCellExtent,
	pub groups: Vec<MobGroup>,
}

pub fn cell_seed(ix: i32, iz: i32) -> u64 {
	mixed(MOB_WORLD_SEED ^ (ix as u32 as u64).rotate_left(17) ^ (iz as u32 as u64).rotate_left(43))
}

pub fn mixed(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

pub fn urban_leaf_arrival_radius(bounds: Aabb3d) -> f32 {
	((bounds.max.x - bounds.min.x).min(bounds.max.z - bounds.min.z) * 0.25).clamp(8.0, 128.0)
}
