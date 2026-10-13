//! Helpers for sampling forest layering and urbanization at a point.

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use chico::{select_cell, ForestExtent, LayeringKind};
use procedural_common::NoiseParams;
use urbanization_cells::{select_kind, UrbanizationExtent, UrbanizationKind};

use crate::generation::MobPlantHost;
use crate::index::urban_leaf_arrival_radius;

pub(crate) fn chico_layers_at(noise: NoiseParams, layering: Option<LayeringKind>, xz: Vec2) -> u8 {
	let position = Vec3::new(xz.x, 0.0, xz.y);
	let (ix, iz) = ForestExtent::cell_index_containing(position);
	let extent = ForestExtent::from_cell_index(ix, iz);
	let layers = match layering {
		Some(kind) => kind.layering().typical_layers(),
		None => select_cell(extent, noise),
	};
	[layers.tufts, layers.understory, layers.lower_canopy, layers.upper_canopy]
		.into_iter()
		.filter(Option::is_some)
		.count() as u8
}

pub(crate) fn richmond_kind_at(
	noise: NoiseParams,
	pinned: Option<UrbanizationKind>,
	xz: Vec2,
) -> UrbanizationKind {
	if let Some(kind) = pinned {
		return kind;
	}
	let position = Vec3::new(xz.x, 0.0, xz.y);
	let (ix, iz) = UrbanizationExtent::cell_index_containing(position);
	select_kind(UrbanizationExtent::from_cell_index(ix, iz), noise)
}

pub(crate) fn host_at(bounds: Aabb3d) -> MobPlantHost {
	MobPlantHost {
		xz: Vec2::new((bounds.min.x + bounds.max.x) * 0.5, (bounds.min.z + bounds.max.z) * 0.5),
		arrival_radius: urban_leaf_arrival_radius(bounds),
	}
}
